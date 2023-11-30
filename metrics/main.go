package main

import (
	db "graph-benchmarks/metrics-server/db"
)

func main() {

	_, err := db.New()
	if err != nil {
		panic(err)
	}
	//lis, err := net.Listen("tcp", fmt.Sprintf("localhost:%d", 9090))
	//if err != nil {
	//	panic(err)
	//}
	//var opts []grpc.ServerOption
	//grpcServer := grpc.NewServer(opts...)
	//PerformanceMetricsServiceServer()
	//RegisterPerformanceMetricsServiceServer(grpcServer, newServer())
	//grpcServer.Serve(lis)
}
