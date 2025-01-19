package main

import (
	"context"
	"fmt"
	"math/rand"
	"testing"

	"github.com/google/uuid"
	_ "github.com/mattn/go-sqlite3"

	"tempo/ent/enttest"
	"tempo/ent/proto/entpb"
	"tempo/ent/task"
	"tempo/ent/user"
)

func TestServiceWithEdges(t *testing.T) {
	ctx := context.Background()
	client := enttest.Open(t, "sqlite3", "file:tempo.db?_fk=1")
	defer client.Close()

	svc := entpb.NewUserService(client)

	create_task := client.Task.
		Create().
		SetTitle("Test Task").
		SetDescription("Test Description").
		SaveX(ctx)

	prefix := fmt.Sprintf("test-user-%d", rand.Int())
	create_user, err := svc.Create(ctx, &entpb.CreateUserRequest{
		User: &entpb.User{
			Name:  prefix,
			Email: prefix + "@testing.tempo.do",
			Tasks: []*entpb.Task{
				{Id: create_task.ID[:]},
			},
		},
	})
	if err != nil {
		t.Fatal("failed to create user via `UserService`", err)
	}

	lookupID, err := uuid.FromBytes(create_user.Id)
	if err != nil {
		t.Fatalf("malformed UUID (%x): %v", create_user.Id, err)
	}
	count, err := client.Task.
		Query().
		Where(
			task.HasUserWith(user.ID(lookupID)),
		).
		Count(ctx)
	if err != nil {
		t.Fatal("failed to count tasks owned by this new user", err)
	}
	if count != 1 {
		t.Fatal("expected exactly 1 task to be owned by this new user")
	}
}

func TestEdgeRetrieval(t *testing.T) {
	ctx := context.Background()
	client := enttest.Open(t, "sqlite3", "file:tempo.db?_fk=1")
	defer client.Close()

	svc := entpb.NewUserService(client)

	prefix := fmt.Sprintf("test-user-%d", rand.Int())
	user := client.User.Create().
		SetName(prefix).
		SetEmail(prefix + "@testing.tempo.do").
		SaveX(ctx)

	client.Task.Create().
		SetTitle("Test Task").
		SetDescription("Test Description").
		SetUser(user).
		SaveX(ctx)

	get, err := svc.Get(ctx, &entpb.GetUserRequest{
		Id: user.ID[:],
	})
	if err != nil {
		t.Fatal("Failed to retrieve the new user", err)
	}
	if len(get.Tasks) != 0 {
		t.Fatal("Edges were not supposed to be retrieved...")
	}

	get, err = svc.Get(ctx, &entpb.GetUserRequest{
		Id:   user.ID[:],
		View: entpb.GetUserRequest_WITH_EDGE_IDS,
	})
	if err != nil {
		t.Fatal("Failed to retrieve the new user", err)
	}
	if len(get.Tasks) != 1 {
		t.Fatal("Expected 1 Task due to `_WITH_EDGE_IDS`, found none.")
	}
}
