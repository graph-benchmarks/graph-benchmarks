package main

import (
	"context"
	"fmt"
	"github.com/go-pg/pg/v10"
	"github.com/google/uuid"
)

type PerformanceMetric struct {
	Id         int64
	RunId      uuid.UUID
	StartTime  int64
	TimeDelta  int64
	PodName    string
	CpuUsage   float64
	RamUsage   float64
	PowerUsage float64
	Interval   int64
}

type MetricsDatabase struct {
	Database         pg.DB
	ConnectionConfig pg.Options
}

func (pm PerformanceMetric) String() string {
	return fmt.Sprintf("PerformanceMetric<%d, %d, %s, %d, %d>", pm.Id, pm.TimeDelta, pm.PodName, pm.CpuUsage, pm.RamUsage)
}

func (db MetricsDatabase) New() {
	//db.Database := pg.Connect(&pg.Options{
	//	User:     "postgres",
	//	Password: "password",
	//})
}

func initDb() bool {
	db := pg.Connect(&pg.Options{
		User:     "postgres",
		Password: "password",
	})

	ctx := context.Background()

	if err := db.Ping(ctx); err != nil {
		panic(err)
	}

	defer func(db *pg.DB) {
		err := db.Close()
		if err != nil {
			fmt.Printf("Unable to close the database connection: %s\n", err)
		}
	}(db)

	return true
}
