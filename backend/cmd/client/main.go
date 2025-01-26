package main

import (
	"context"
	"fmt"
	"log"
	"math/rand"

	"tempo/ent/proto/tempo"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/status"
)

func main() {
	// Open a connection to the server.
	conn, err := grpc.NewClient(":8367", grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		log.Fatalf("failed connecting to server: %s", err)
	}
	defer conn.Close()

	// Create a User service Client on the connection.
	client_user := tempo.NewUserServiceClient(conn)

	// Ask the server to create a random User.
	ctx := context.Background()
	user := randomUser()
	created, err := client_user.Create(ctx, &tempo.CreateUserRequest{
		User: user,
	})
	if err != nil {
		se, _ := status.FromError(err)
		log.Fatalf("failed creating user: status=%s message=%s", se.Code(), se.Message())
	}
	log.Printf("user created with id: %x", created.Id)

	// On a separate RPC invocation, retrieve the user we saved previously.
	get, err := client_user.Get(ctx, &tempo.GetUserRequest{
		Id: created.Id,
	})
	if err != nil {
		se, _ := status.FromError(err)
		log.Fatalf("failed retrieving user: status=%s message=%s", se.Code(), se.Message())
	}
	log.Printf("retrieved user with id=%x: %v", get.Id, get)
}

func randomUser() *tempo.User {
	return &tempo.User{
		Name:  fmt.Sprintf("user-%d", rand.Int()),
		Email: fmt.Sprintf("user-%d@testing.tempo.do", rand.Int()),
	}
}
