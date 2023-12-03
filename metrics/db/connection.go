package db

import (
	"context"
	"fmt"
	"github.com/go-pg/pg/v10"
	"github.com/go-pg/pg/v10/orm"
	"graph-benchmarks/metrics-server/config"
	"log"
)

type Handler struct {
	db *pg.DB
}

func New(cfg config.SqlConfig) (Handler, error) {

	opts := pg.Options{
		Addr:     fmt.Sprintf("%s:%d", cfg.Host, cfg.Port),
		User:     cfg.Username,
		Password: cfg.Password,
		Database: cfg.Database,
	}

	db := pg.Connect(&opts)

	// Ping the database
	ctx := context.Background()
	if err := db.Ping(ctx); err != nil {
		log.Panicf("Unable to connect to the database: %s", err)
	}

	// Create table if not exists
	err := db.Model((*PerformanceMetric)(nil)).CreateTable(&orm.CreateTableOptions{
		IfNotExists: true,
	})

	if err != nil {
		log.Panicf("Unable to create metrics table: %s", err)
	}

	return Handler{db: db}, nil

}

func (handler *Handler) Close() {
	handler.db.Close()
}
