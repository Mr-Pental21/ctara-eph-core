package dhruv

import (
	"fmt"
	"runtime"

	"ctara-dhruv-core/bindings/go-open/internal/cabi"
)

type Engine struct {
	h cabi.EngineHandle
}

type LSK struct {
	h cabi.LskHandle
}

type EOP struct {
	h cabi.EopHandle
}

type Config struct {
	h cabi.ConfigHandle
}

func APIVersion() uint32 {
	return cabi.APIVersion()
}

func VerifyABI() error {
	if got := APIVersion(); got != ExpectedAPIVersion {
		return fmt.Errorf("verify_abi failed: expected ABI v%d, got v%d", ExpectedAPIVersion, got)
	}
	return nil
}

func LoadConfig(path string) (*Config, error) {
	h, st := cabi.LoadConfig(path)
	if err := statusErr("config_load", st); err != nil {
		return nil, err
	}
	cfg := &Config{h: h}
	runtime.SetFinalizer(cfg, (*Config).Close)
	return cfg, nil
}

func ClearActiveConfig() error {
	return statusErr("config_clear_active", cabi.ConfigClearActive())
}

func (c *Config) Close() error {
	if c == nil {
		return nil
	}
	runtime.SetFinalizer(c, nil)
	return statusErr("config_free", c.h.Free())
}

func NewEngine(cfg EngineConfig) (*Engine, error) {
	h, st, prior := cabi.CreateEngine(cfg)
	if err := wrapStatus("engine_new", st, prior); err != nil {
		return nil, err
	}
	e := &Engine{h: h}
	runtime.SetFinalizer(e, (*Engine).Close)
	return e, nil
}

func (e *Engine) Close() error {
	if e == nil {
		return nil
	}
	runtime.SetFinalizer(e, nil)
	return statusErr("engine_free", e.h.Free())
}

func (e *Engine) Query(q QueryRequest) (QueryResult, error) {
	out, st := cabi.QueryEngineRequest(e.h, q)
	return out, statusErr("engine_query", st)
}

func QueryOnce(cfg EngineConfig, q Query) (StateVector, error) {
	out, st, prior := cabi.QueryOnce(cfg, q)
	return out, wrapStatus("query_once", st, prior)
}

func LoadLSK(path string) (*LSK, error) {
	h, st := cabi.LoadLSK(path)
	if err := statusErr("lsk_load", st); err != nil {
		return nil, err
	}
	l := &LSK{h: h}
	runtime.SetFinalizer(l, (*LSK).Close)
	return l, nil
}

func (l *LSK) Close() error {
	if l == nil {
		return nil
	}
	runtime.SetFinalizer(l, nil)
	return statusErr("lsk_free", l.h.Free())
}

func LoadEOP(path string) (*EOP, error) {
	h, st := cabi.LoadEOP(path)
	if err := statusErr("eop_load", st); err != nil {
		return nil, err
	}
	e := &EOP{h: h}
	runtime.SetFinalizer(e, (*EOP).Close)
	return e, nil
}

func (e *EOP) Close() error {
	if e == nil {
		return nil
	}
	runtime.SetFinalizer(e, nil)
	return statusErr("eop_free", e.h.Free())
}

func CartesianToSpherical(position [3]float64) (SphericalCoords, error) {
	out, st := cabi.CartesianToSpherical(position)
	return out, statusErr("cartesian_to_spherical", st)
}
