package k8s

import (
	"fmt"
	"graph-benchmarks/metrics-server/config"
	"graph-benchmarks/metrics-server/db"
	"time"
)

type MetricsPollingWorker struct {
	db        db.Handler
	k8sClient Client
	runId     int64
	interval  int64
	podNames  []string
	signal    chan struct{}
}

func New(sqlCfg config.SqlConfig, k8sCfg config.K8sConfig, runId int64, interval int64, podNames []string) (MetricsPollingWorker, error) {
	db, err := db.New(sqlCfg)
	if err != nil {
		panic(err)
	}
	k8sClient, err := NewClients()
	if err != nil {
		panic(err)
	}

	return MetricsPollingWorker{
		db:        db,
		k8sClient: k8sClient,
		interval:  interval,
		podNames:  podNames,
	}, nil
}

func (w *MetricsPollingWorker) Start() {
	ticker := time.NewTicker(time.Duration(w.interval) * time.Millisecond)

	go func() {
		now := time.Now().UnixMilli()
		for {
			select {
			case <-ticker.C:
				for _, name := range w.podNames {
					metrics, err := w.k8sClient.GetMetrics(name)
					if err != nil {
						fmt.Printf("Failed to get metrics from pod: %s\n", name)
					}
					cpuUsage := metrics.Containers[0].Usage.Cpu().AsApproximateFloat64()
					ramUsage, _ := metrics.Containers[0].Usage.Memory().AsInt64()
					fmt.Printf("cpu: %f, mem: %d\n", cpuUsage, ramUsage)

					pm := db.PerformanceMetric{
						Id:         0,
						RunId:      w.runId,
						StartTime:  now,
						TimeDelta:  w.interval,
						PodName:    name,
						CpuUsage:   cpuUsage,
						RamUsage:   float64(ramUsage),
						PowerUsage: 0,
						Interval:   w.interval,
					}

					err = w.db.NewRecord(&pm)
					if err != nil {
						fmt.Printf("Unable to write to db: %v\n", err)
						return
					}

				}
			case <-w.signal:
				ticker.Stop()
				return
			}
		}
	}()
}

func (w *MetricsPollingWorker) Stop() {
	close(w.signal)
	fmt.Printf("Metrics collection worker is shutting down.")
}