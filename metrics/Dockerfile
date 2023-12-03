# syntax=docker/dockerfile:1

FROM golang:1.21-alpine

WORKDIR /app

# Copy and Download Go modules
COPY go.mod go.sum ./
RUN go mod download
COPY . .

# Build
RUN CGO_ENABLED=0 GOOS=linux go build -o /metrics-server

EXPOSE 9090

ENTRYPOINT ["/metrics-server"]