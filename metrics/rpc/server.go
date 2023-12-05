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

func (s *Rpc) StartServer(grpcCfg config.GrpcConfig, k8sCfg config.K8sConfig, sqlCfg config.SqlConfig) error {

	lis, err := net.Listen("tcp", fmt.Sprintf("%s:%d", grpcCfg.Host, grpcCfg.Port))

	if err != nil {
		log.Fatalf("Failed to listen: %v", err)
	}

	var opts []grpc.ServerOption
	s.handler = grpc.NewServer(opts...)
	//reflection.Register(s.handler)
	metricsServer := MetricsServer{}
	metricsServer.SqlConfig = sqlCfg
	RegisterMetricsCollectorServer(s.handler, &metricsServer)

	// Start grpc server
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
