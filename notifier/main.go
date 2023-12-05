package main

import (
	"github.com/gofiber/contrib/websocket"
	"github.com/gofiber/fiber/v2"
)

var events chan bool

type Event struct {
	Status bool `json:"status"`
}

func main() {
	app := fiber.New()
	events = make(chan bool)

	app.Post("/starting", func(c *fiber.Ctx) error {
		events <- true
		return c.SendStatus(200)
	})

	app.Post("/stopping", func(c *fiber.Ctx) error {
		events <- false
		return c.SendStatus(200)
	})

	app.Use("/ws", func(c *fiber.Ctx) error {
		if websocket.IsWebSocketUpgrade(c) {
			c.Locals("allowed", true)
			return c.Next()
		}
		return fiber.ErrUpgradeRequired
	})

	app.Get("/ws", websocket.New(func(c *websocket.Conn) {
		for {
			status := <-events
			c.WriteJSON(Event{
				Status: status,
			})
		}
	}))

	app.Listen(":8080")
}
