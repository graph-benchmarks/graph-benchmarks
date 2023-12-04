package config

import (
	"fmt"
	"os"

	"gopkg.in/yaml.v3"
)

func FromFile(path string) (GlobalConfig, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		panic(err)
	}
	cfg := &GlobalConfig{}
	err = yaml.Unmarshal(data, cfg)
	if err != nil {
		fmt.Printf("Unmarshal: %v\n", err)
	}

	return *cfg, nil
}

func FromCmd(sqlHost, sqlUsername, sqlPassword, sqlDb, grpcHost string, sqlPort, grpcPort int64, k8sAuth bool) GlobalConfig {
	cfg := GlobalConfig{
		Sql: SqlConfig{
			Host:     sqlHost,
			Port:     sqlPort,
			Username: sqlUsername,
			Password: sqlPassword,
			Database: sqlDb,
		},
		Grpc: GrpcConfig{
			Host: grpcHost,
			Port: grpcPort,
		},
		K8s: K8sConfig{InClusterConfig: k8sAuth},
	}
	return cfg
}
