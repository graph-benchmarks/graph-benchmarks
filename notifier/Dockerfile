FROM golang:1.21-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . /app
RUN CGO_ENABLED=0 GOOS=linux go build -o notifier

FROM alpine:latest AS production
COPY --from=builder /app .
EXPOSE 9090
ENTRYPOINT ["./notifier"]