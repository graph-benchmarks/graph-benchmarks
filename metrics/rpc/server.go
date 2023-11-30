package rpc

import (
	"context"
)

type Server struct {
	UnimplementedMetricsCollectorServer
}

func New() *Server {
	panic("Unimplemented!")
}

func StartServer() error {
	panic("Unimplemented!")
}

func StopServer() error {
	panic("Unimplemented!")
}

func (s *Server) StartRecording(ctx context.Context, startSignal *Start) (*Ack, error) {
	panic("Unimplemented!")
}

func (s *Server) StopRecording(ctx context.Context, stopSignal *Stop) (*Ack, error) {
	panic("Unimplemented!")
}
