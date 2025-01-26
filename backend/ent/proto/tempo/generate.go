package tempo

//go:generate protoc -I=.. --go_out=.. --go-grpc_out=.. --go_opt=paths=source_relative --go-grpc_opt=paths=source_relative --entgrpc_out=.. --entgrpc_opt=paths=source_relative,schema_path=../../schema tempo/tempo.proto
//go:generate protoc -I=. --dart_out=grpc:../../../../app/lib/proto tempo.proto
