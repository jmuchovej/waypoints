import uvicorn
import typer

cli = typer.Typer(no_args_is_help=True)

@cli.command()
def serve() -> None:
    uvicorn.run(
        app="tempo.server:app",
        port=8676,
        reload=True,
        access_log=True,
    )

@cli.command()
def build_db() -> None:
    from tempo.common.clients import db
    from sqlmodel import SQLModel

    try:
        from tempo.transcription.models import Models
    except ImportError:
        print("Attempted to import `tempo.transcription.models` and failed!")

    SQLModel.metadata.create_all(db)


if __name__ == "__main__":
    cli()
