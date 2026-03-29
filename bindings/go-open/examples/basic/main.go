package main

import (
	"fmt"
	"log"
	"os"

	"ctara-dhruv-core/bindings/go-open/dhruv"
)

func main() {
	spk := os.Getenv("DHRUV_SPK_PATH")
	lsk := os.Getenv("DHRUV_LSK_PATH")
	if spk == "" || lsk == "" {
		log.Fatal("set DHRUV_SPK_PATH and DHRUV_LSK_PATH")
	}

	if err := dhruv.VerifyABI(); err != nil {
		log.Fatalf("ABI check failed: %v", err)
	}

	engine, err := dhruv.NewEngine(dhruv.EngineConfig{
		SpkPaths:         []string{spk},
		LskPath:          lsk,
		CacheCapacity:    128,
		StrictValidation: false,
	})
	if err != nil {
		log.Fatalf("engine init: %v", err)
	}
	defer engine.Close()

	result, err := engine.Query(dhruv.QueryRequest{
		Target:     301,
		Observer:   399,
		Frame:      1,
		TimeKind:   dhruv.QueryTimeJDTDB,
		EpochTdbJD: 2451545.0,
		OutputMode: dhruv.QueryOutputCartesian,
	})
	if err != nil {
		log.Fatalf("query failed: %v", err)
	}
	if result.State == nil {
		log.Fatal("query did not return cartesian state")
	}

	fmt.Printf(
		"Moon position km: [%.3f %.3f %.3f]\n",
		result.State.PositionKm[0],
		result.State.PositionKm[1],
		result.State.PositionKm[2],
	)
}
