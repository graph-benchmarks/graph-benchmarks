package db

import (
	"context"
	"github.com/go-pg/pg/v10"
	"github.com/go-pg/pg/v10/orm"
	"log"
)

type Handler struct {
	db *pg.DB
}

func New() (Handler, error) {
	// TODO(caesar): update to use external config
	opts := pg.Options{
		User:     "postgres",
		Password: "password",
		Database: "postgres",
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
