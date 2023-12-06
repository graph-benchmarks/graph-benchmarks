package db

import (
	"fmt"
	"log/slog"
)

type PerformanceMetric struct {
	Id         int64
	RunId      int64
	StartTime  int64
	TimeDelta  int64
	PodName    string
	CpuUsage   float64
	RamUsage   float64
	PowerUsage float64
	Interval   int64
}

func (pm PerformanceMetric) String() string {
	return fmt.Sprintf("PerformanceMetric<%d, %d, %s, %f, %f>", pm.Id, pm.TimeDelta, pm.PodName, pm.CpuUsage, pm.RamUsage)
}

func (handler *Handler) NewRecord(pm *PerformanceMetric) error {
	_, err := handler.db.Model(pm).Insert()
	if err != nil {
		slog.Error("Unable to insert record: %s", err)
	}
	return nil
}

func (handler *Handler) GetRecord() (PerformanceMetric, error) {
	// TODO(caesar): implement get pm record function
	panic("Unimplemented!")
}
