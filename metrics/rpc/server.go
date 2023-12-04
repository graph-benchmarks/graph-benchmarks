package rpc

import (
	"fmt"
	"graph-benchmarks/metrics-server/config"
	"log"
	"net"

	"google.golang.org/grpc"
)

type Rpc struct {
	handler *grpc.Server
}

func (s *Rpc) StartServer(host string, port int64, k8sCfg config.K8sConfig, sqlCfg config.SqlConfig) error {
	lis, err := net.Listen("tcp", fmt.Sprintf(":%d", port))

	if err != nil {
		log.Fatalf("Failed to listen: %v", err)
	}

	var opts []grpc.ServerOption
	s.handler = grpc.NewServer(opts...)
	metricsServer := MetricsServer{}
	metricsServer.SqlConfig = sqlCfg
	RegisterMetricsCollectorServer(s.handler, &metricsServer)

	// Start grpc server
	log.Println("listening!")
	if err := s.handler.Serve(lis); err != nil {
		log.Fatalf("failed to serve: %v", err)
		return err
	}
	return nil
}

func (s *Rpc) StopServer() {
	s.handler.Stop()
	log.Println("RPC server stopped.")
}
