if __name__ == "__main__":
    import uvicorn
    uvicorn.run(
        app="tempo.transcription:app",
        port=8676,
        reload=True,
        access_log=True,
    )
