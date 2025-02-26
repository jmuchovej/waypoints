import 'package:flutter/material.dart';
import "package:flutter_riverpod/flutter_riverpod.dart";

import "package:transcription/screens/player.dart";

void main() {
  runApp(const ProviderScope(child: TranscriptionService()));
}

class TranscriptionService extends StatelessWidget {
  const TranscriptionService({super.key});

  // This widget is the root of your application.
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: "Transcription",
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.blueGrey),
      ),
      home: const PlayerScreen(
        audioURL: "assets/2025-01-13-meeting.m4a",
        transcriptURL: "assets/2025-01-13-transcript.json",
      ),
    );
  }
}
