import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:transcription/service.dart";
import "package:transcription/widgets/playback.dart";
import "package:transcription/widgets/transcript.dart";

final playerProvider = FutureProvider.family((
  ref,
  (String audioURL, String transcriptURL) args,
) async {
  final audio = await ref.watch(audioServiceProvider(args.$1).future);
  final transcript = await ref.watch(transcriptProvider(args.$2).future);
  return {"audio": audio, "transcript": transcript};
});

class PlayerScreen extends ConsumerWidget {
  final String audioURL;
  final String transcriptURL;

  const PlayerScreen({
    super.key,
    required this.audioURL,
    required this.transcriptURL,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final provider = ref.watch(playerProvider((audioURL, transcriptURL)));

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
        title: Text("Player"),
      ),
      body: provider.when(
        data: (providers) {
          final audio = providers["audio"] as AudioService;
          final transcript = providers["transcript"] as TranscriptJSON;
          return Column(
            children: [
              Expanded(
                child: Center(
                  child: TranscriptWidget(
                    player: audio.player,
                    transcript: transcript,
                  ),
                ),
              ),
              PlaybackWidget(service: audio),
            ],
          );
        },
        loading:
            () => Column(
              children: [
                Expanded(child: Center(child: CircularProgressIndicator())),
              ],
            ),
        error:
            (err, _) => Column(
              children: [
                Expanded(
                  child: Center(
                    child: Text("Error loading audio or transcript..."),
                  ),
                ),
              ],
            ),
      ),
    );
  }
}
