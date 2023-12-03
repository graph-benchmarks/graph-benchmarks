package config

type SqlConfig struct {
	Host     string `yaml:"host"`
	Port     int64  `yaml:"port"`
	Username string `yaml:"username"`
	Password string `yaml:"password"`
	Database string `yaml:"database"`
}

type GrpcConfig struct {
	Host string `yaml:"host"`
	Port int64  `yaml:"port"`
}

type K8sConfig struct {
	InClusterConfig bool `yaml:"in_cluster_config"`
}

type GlobalConfig struct {
	Sql  SqlConfig  `yaml:"postgresql"`
	Grpc GrpcConfig `yaml:"grpc"`
	K8s  K8sConfig  `yaml:"k8s"`
}
