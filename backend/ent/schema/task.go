package schema

import (
	"entgo.io/contrib/entproto"
	"entgo.io/ent"
	"entgo.io/ent/schema"
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
	var statusMap = make(map[string]int32)
	for index := 0; index < len(Status); index += 2 {
		statusMap[Status[index+1]] = int32(index)
	}
	return []ent.Field{
		field.UUID("id", uuid.UUID{}).
			Default(uuid.New).
			Annotations(entproto.Field(1)),
		field.String("title").
			Annotations(entproto.Field(2)),
		field.String("description").
			Annotations(entproto.Field(3)),
		field.Enum("status").
			NamedValues(Status[:]...).
			Default("todo").
			Annotations(
				entproto.Field(4),
				entproto.Enum(statusMap),
			),
	}
}

// Edges of the Task.
func (Task) Edges() []ent.Edge {
	return []ent.Edge{
		edge.From("user", User.Type).
			Ref("tasks").
			Unique().
			Annotations(entproto.Field(5)),
	}
}

func (Task) Annotations() []schema.Annotation {
	return []schema.Annotation{
		entproto.Message(
			entproto.PackageName("tempo"),
		),
		entproto.Service(),
	}
}
