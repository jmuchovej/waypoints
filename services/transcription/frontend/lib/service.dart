import "dart:convert";
import "package:flutter/services.dart";
import "package:just_audio/just_audio.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
// import "package:http/http.dart" as http;

class AudioService {
  final _player = AudioPlayer();

  AudioPlayer get player => _player;

  Future<void> setSource(String path) async {
    await _player.setAsset(path);
  }

  void dispose() {
    _player.dispose();
  }
}

final audioServiceProvider = FutureProvider.family<AudioService, String>((
  Ref ref,
  String url,
) async {
  final service = AudioService();
  await service.setSource(url);
  ref.onDispose(() => service.dispose());
  return service;
});

class Utterance {
  final String text;
  final int start;
  final int end;
  final String speaker;
  final String? channel;
  final double confidence;

  Utterance({
    required this.text,
    required this.start,
    required this.end,
    required this.speaker,
    required this.confidence,
    this.channel,
  });
}

typedef TranscriptJSON = List<Map<String, dynamic>>;
final transcriptProvider = FutureProvider.family<TranscriptJSON, String>((
  Ref ref,
  String url,
) async {
  final data = await rootBundle.loadString(url);
  return TranscriptJSON.from(json.decode(data)["utterances"]);

  // TODO once URLs are live, use this implementation.
  // final response = await http.get(Uri.parse(url));

  // if (response.statusCode != 200) {
  //   throw Exception("Failed to load Transcript.");
  // }

  // return TranscriptJSON.from(json.decode(response.body).utterances);
});
