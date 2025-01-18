package main

import (
	"testing"

	"tempo/ent/proto/entpb"
)

func TestUserProto(t *testing.T) {
	user := entpb.User{
		Name:  "username",
		Email: "username@testing.tempo.do",
	}
	if user.GetName() != "username" {
		t.Fatal("expected user name to be `username`")
	}
	if user.GetEmail() != "username@testing.tempo.do" {
		t.Fatal("expected email address to be `username@testing.tempo.do`")
	}
}
