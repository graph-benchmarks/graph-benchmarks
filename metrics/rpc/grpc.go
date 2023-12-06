package rpc

import (
	"context"
	"graph-benchmarks/metrics-server/config"
	"graph-benchmarks/metrics-server/k8s"
	"log"
)

type MetricsServer struct {
	UnimplementedMetricsCollectorServer
	k8sConfig config.K8sConfig
	sqlConfig config.SqlConfig
	worker    k8s.MetricsPollingWorker
}

func New(k8sCfg config.K8sConfig, sqlConfig config.SqlConfig) MetricsServer {
	return MetricsServer{
		UnimplementedMetricsCollectorServer: UnimplementedMetricsCollectorServer{},
		k8sConfig:                           k8sCfg,
		sqlConfig:                           sqlConfig,
	}
}

func (s *MetricsServer) StartRecording(ctx context.Context, req *Start) (*Ack, error) {
	log.Printf("Received start recording request: %s", req)
	var err error
	s.worker, err = k8s.New(s.sqlConfig, s.k8sConfig, req.RunId, int64(req.Interval), req.PodIds)
	if err != nil {
		return &Ack{
			Status:  false,
			Message: "Unable to start metrics worker, check metrics server log.",
		}, err
	}
	s.worker.Start()

	return &Ack{
		Status:  true,
		Message: "success",
	}, nil
}

func (s *MetricsServer) StopRecording(ctx context.Context, req *Stop) (*Ack, error) {
	log.Printf("Received stop recording request: %s", req)
	s.worker.Stop()

	return &Ack{
		Status:  true,
		Message: "success",
	}, nil
}
