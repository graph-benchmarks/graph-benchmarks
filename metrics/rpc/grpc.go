package rpc

import (
	"context"
	"graph-benchmarks/metrics-server/config"
	"graph-benchmarks/metrics-server/k8s"
	"log"
)

type MetricsServer struct {
	UnimplementedMetricsCollectorServer
	K8sConfig config.K8sConfig
	SqlConfig config.SqlConfig
	Worker    k8s.MetricsPollingWorker
}

func New(k8sCfg config.K8sConfig, sqlConfig config.SqlConfig) MetricsServer {
	return MetricsServer{
		UnimplementedMetricsCollectorServer: UnimplementedMetricsCollectorServer{},
		K8sConfig:                           k8sCfg,
		SqlConfig:                           sqlConfig,
	}
}

func (s *MetricsServer) StartRecording(ctx context.Context, req *Start) (*Ack, error) {
	log.Printf("Received start recording request: %s", req)
	var err error
	s.Worker, err = k8s.New(s.SqlConfig, s.K8sConfig, req.RunId, int64(req.Interval), req.PodIds)
	if err != nil {
		return nil, err
	}
	s.Worker.Start()

	return &Ack{
		Status:  true,
		Message: "success",
	}, nil
}

func (s *MetricsServer) StopRecording(ctx context.Context, req *Stop) (*Ack, error) {
	log.Printf("Received stop recording request: %s", req)
	s.Worker.Stop()

	return &Ack{
		Status:  true,
		Message: "success",
	}, nil
}
