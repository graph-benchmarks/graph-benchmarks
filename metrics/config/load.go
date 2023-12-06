package config

import (
	"log"
	"os"

	"gopkg.in/yaml.v3"
)

func FromFile(path string) (GlobalConfig, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return GlobalConfig{}, err
	}
	cfg := &GlobalConfig{}
	err = yaml.Unmarshal(data, cfg)
	if err != nil {
		log.Fatalf("Unmarshal: %v\n", err)
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
