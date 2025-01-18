package schema

import (
	"entgo.io/ent"
	"entgo.io/ent/schema/edge"
	"entgo.io/ent/schema/field"
	"github.com/google/uuid"
)

// Task holds the schema definition for the Task entity.
type Task struct {
	ent.Schema
}

var Status = [...]string{
	"Todo", "todo",
	"Pending", "pending",
	"Scheduled", "scheduled",
	"Started", "started",
	"Paused", "paused",
	"WorkInProgress", "wip",
	"Waiting", "waiting",
	"Stopped", "stopped",
	"Completed", "completed",
	"Cancelled", "cancelled",
}

// Fields of the Task.
func (Task) Fields() []ent.Field {
	return []ent.Field{
		field.UUID("id", uuid.UUID{}).Default(uuid.New),
		field.String("title"),
		field.String("description"),
		field.Enum("status").
			NamedValues(Status[:]...).
			Default("todo"),
	}
}

// Edges of the Task.
func (Task) Edges() []ent.Edge {
	return []ent.Edge{
		edge.From("user", User.Type).
			Ref("tasks").
			Unique(),
	}
}
