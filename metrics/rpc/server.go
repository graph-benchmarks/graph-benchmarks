package rpc

import (
	"fmt"
	"google.golang.org/grpc"
	"google.golang.org/grpc/reflection"
	"graph-benchmarks/metrics-server/config"
	"log"
	"net"
)

type Rpc struct {
	handler *grpc.Server
}

func (s *Rpc) StartServer(host string, port int64, k8sCfg config.K8sConfig, sqlCfg config.SqlConfig) error {
	lis, err := net.Listen("tcp", fmt.Sprintf("%s:%d", host, port))

	if err != nil {
		log.Fatalf("Failed to listen: %v", err)
	}

	var opts []grpc.ServerOption
	s.handler = grpc.NewServer(opts...)
	metricsServer := MetricsServer{}
	RegisterMetricsCollectorServer(s.handler, &metricsServer)
	reflection.Register(s.handler)

	// Start grpc server
	go s.handler.Serve(lis)

	if err != nil {
		return err
	}

	log.Printf("RPC server started on: %s:%d\n", host, port)
	return nil
}

func (s *Rpc) StopServer() {
	s.handler.Stop()
	log.Println("RPC server stopped.")
}
