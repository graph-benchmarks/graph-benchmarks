package main

import (
	"flag"
	"graph-benchmarks/metrics-server/config"
	"graph-benchmarks/metrics-server/rpc"
	"log"
)

func main() {

	cfgPath := flag.String("config", "", "the config file (highest priority on params)")

	psqlHost := flag.String("psql-host", "127.0.0.1", "the hostname of PostgreSQL")
	psqlPort := flag.Int64("psql-port", 5432, "the port of PostgreSQL")
	psqlUsername := flag.String("psql-username", "postgres", "the username of PostgreSQL")
	psqlPassword := flag.String("psql-password", "password", "the password of PostgreSQL")
	psqlDatabase := flag.String("psql-db", "postgres", "the database name of PostgreSQL")

	grpcHost := flag.String("grpc-host", "0.0.0.0", "the host of grpc server")
	grpcPort := flag.Int64("grpc-port", 9090, "the port of grpc server")

	k8sAuthMethod := flag.Bool("k8s-auth", true, "the auth method of k8s API (only in-cluster implemented)")

	flag.Parse()

	cfg := config.GlobalConfig{}
	if *cfgPath == "" {
		cfg = config.FromCmd(*psqlHost, *psqlUsername, *psqlPassword, *psqlDatabase, *grpcHost, *psqlPort, *grpcPort, *k8sAuthMethod)
	} else {
		var err error
		cfg, err = config.FromFile(*cfgPath)
		if err != nil {
			panic(err)
		}
	}
	log.Println(cfg)

	rpc := rpc.Rpc{}
	rpc.StartServer(cfg.Grpc.Host, cfg.Grpc.Port, cfg.K8s, cfg.Sql)
}
