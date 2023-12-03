package k8s

import (
	"context"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/rest"
	"k8s.io/metrics/pkg/apis/metrics/v1beta1"
	metrics "k8s.io/metrics/pkg/client/clientset/versioned"
)

type Client struct {
	restConfig    *rest.Config
	clientset     *kubernetes.Clientset
	metricsClient *metrics.Clientset
}

func NewClients() (Client, error) {

	// Creates the in-cluster config
	config, err := rest.InClusterConfig()
	if err != nil {
		panic(err.Error())
	}

	// Creates the clientset
	clientset, err := kubernetes.NewForConfig(config)
	if err != nil {
		panic(err.Error())
	}

	// Creates the metrics client
	mc, err := metrics.NewForConfig(config)

	return Client{
		restConfig:    config,
		clientset:     clientset,
		metricsClient: mc,
	}, err

}

func (c *Client) GetMetrics(name string) (*v1beta1.PodMetrics, error) {
	//metrics, err := c.metricsClient.MetricsV1beta1().PodMetricses(metav1.NamespaceAll).Get(context.TODO(), name, metav1.GetOptions{})
	metrics, err := c.metricsClient.MetricsV1beta1().PodMetricses("default").Get(context.TODO(), name, metav1.GetOptions{})
	if err != nil {
		return nil, err
	}
	return metrics, nil
}
