package main

import (
	"context"
	"log"
	"net"

	"tempo/ent"
	"tempo/ent/proto/entpb"

	_ "github.com/mattn/go-sqlite3"
	"google.golang.org/grpc"
)

func main() {
	// Initialize an ent client.
	client, err := ent.Open("sqlite3", "file:tempo.db?_fk=1")
	if err != nil {
		log.Fatalf("failed opening connection to sqlite: %v", err)
	}
	defer client.Close()

	// Run the migration tool (creating tables, etc).
	if err := client.Schema.Create(context.Background()); err != nil {
		log.Fatalf("failed creating schema resources: %v", err)
	}

	// Initialize the generated User service.
	svc_user := entpb.NewUserService(client)

	// Create a new gRPC server (you can wire multiple services to a single server).
	server := grpc.NewServer()

	// Register the User service with the server.
	entpb.RegisterUserServiceServer(server, svc_user)

	// Open port 8367 for listening to traffic.
	lis, err := net.Listen("tcp", ":8367")
	if err != nil {
		log.Fatalf("failed listening: %s", err)
	}

	// Listen for traffic indefinitely.
	if err := server.Serve(lis); err != nil {
		log.Fatalf("server ended: %s", err)
	}
}
