package process

import (
	"connectrpc.com/connect"
	"github.com/e2b-dev/infra/packages/envd/internal/processio"
	"github.com/e2b-dev/infra/packages/envd/internal/services/process/handler"
	rpc "github.com/e2b-dev/infra/packages/envd/internal/services/spec/process"
	"github.com/e2b-dev/infra/packages/envd/internal/utils"
	"testing"
)

func fixtureService() *Service {
	return &Service{recoverable: make(map[string]*handler.Handler), processes: utils.NewMap[uint32, *handler.Handler]()}
}
func TestRecoverableSelectorCannotFallBackToPIDOrTag(t *testing.T) {
	s := fixtureService()
	j, err := processio.New()
	if err != nil {
		t.Fatal(err)
	}
	tag := "fixture"
	p := &handler.Handler{IO: j, Tag: &tag}
	s.recoverable[j.ID()] = p
	s.storeProcess(123, p)
	for _, selector := range []*rpc.ProcessSelector{
		{Selector: &rpc.ProcessSelector_Pid{Pid: 123}},
		{Selector: &rpc.ProcessSelector_Tag{Tag: tag}},
	} {
		if _, err := s.getProcess(selector); connect.CodeOf(err) != connect.CodeFailedPrecondition {
			t.Fatal(err)
		}
	}
	if got, err := s.getProcess(&rpc.ProcessSelector{Selector: &rpc.ProcessSelector_Incarnation{Incarnation: j.ID()}}); err != nil || got != p {
		t.Fatal(got, err)
	}
	if _, err := s.getProcess(&rpc.ProcessSelector{Selector: &rpc.ProcessSelector_Incarnation{Incarnation: "expired"}}); connect.CodeOf(err) != connect.CodeNotFound {
		t.Fatal(err)
	}
}
func TestPIDReuseDoesNotLoseNewProcessOrOldIncarnation(t *testing.T) {
	s := fixtureService()
	j, err := processio.New()
	if err != nil {
		t.Fatal(err)
	}
	old, newProcess := &handler.Handler{IO: j}, &handler.Handler{}
	s.recoverable[j.ID()] = old
	s.storeProcess(123, old)
	s.storeProcess(123, newProcess)
	s.retireProcess(123, old)
	if got, _ := s.processes.Load(123); got != newProcess {
		t.Fatal("new PID occupant deleted")
	}
	if got, err := s.getProcess(&rpc.ProcessSelector{Selector: &rpc.ProcessSelector_Incarnation{Incarnation: j.ID()}}); err != nil || got != old {
		t.Fatal(got, err)
	}
}
func TestRecoveryCapacityMustBeReservedBeforeProcessCreation(t *testing.T) {
	s := fixtureService()
	var releases []func()
	for range maxRecoverableProcesses {
		release, err := s.reserveRecovery(true)
		if err != nil {
			t.Fatal(err)
		}
		releases = append(releases, release)
	}
	if _, err := s.reserveRecovery(true); connect.CodeOf(err) != connect.CodeResourceExhausted {
		t.Fatal(err)
	}
	if release, err := s.reserveRecovery(false); err != nil || release != nil {
		t.Fatal("legacy API cannot reserve a recovery slot")
	}
	releases[0]()
	release, err := s.reserveRecovery(true)
	if err != nil {
		t.Fatal(err)
	}
	release()
	for _, release := range releases[1:] {
		release()
	}
	if s.recoverableCount != 0 {
		t.Fatal(s.recoverableCount)
	}
}
