import "dart:math";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:just_audio/just_audio.dart";
import "package:iconify/icons/tabler.dart";
import "package:iconify/iconify.dart";
import "package:transcription/service.dart";
import "package:transcription/utils/duration.dart" as durationUtils;

class PlaybackWidget extends ConsumerWidget {
  final AudioService service;

  const PlaybackWidget({super.key, required this.service});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final player = service.player;

    return Container(
      alignment: Alignment.bottomCenter,
      margin: EdgeInsets.all(12),
      decoration: BoxDecoration(
        border: Border.all(
          color: Theme.of(context).colorScheme.inversePrimary,
          width: 3,
        ),
        borderRadius: BorderRadius.all(Radius.circular(8)),
      ),
      child: Column(
        children: [
          StreamBuilder<Duration>(
            stream: player.positionStream,
            builder: _playbackProgress(player),
          ),
          Padding(
            padding: EdgeInsets.symmetric(horizontal: 24.0),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              mainAxisSize: MainAxisSize.max,
              children: [
                StreamBuilder<Duration>(
                  stream: player.positionStream,
                  builder: _nowTime(player),
                ),
                StreamBuilder<Duration>(
                  stream: player.positionStream,
                  builder: _maxTime(player),
                ),
              ],
            ),
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceAround,
            mainAxisSize: MainAxisSize.max,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  StreamBuilder<Duration>(
                    stream: player.positionStream,
                    builder: _playbackAdvanceButton(player, "bwd"),
                  ),
                  StreamBuilder<PlayerState>(
                    stream: player.playerStateStream,
                    builder: _playButton(player),
                  ),
                  StreamBuilder<Duration>(
                    stream: player.positionStream,
                    builder: _playbackAdvanceButton(player, "fwd"),
                  ),
                  Container(width: 16),
                  StreamBuilder<PlayerState>(
                    stream: player.playerStateStream,
                    builder: _playbackSpeedButton(player),
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
    );
  }
}

AsyncWidgetBuilder<Duration> _playbackAdvanceButton(
  AudioPlayer player,
  String direction,
) {
  // TODO support customizing forward and backward skip durations
  Widget builder(BuildContext context, AsyncSnapshot<Duration> snapshot) {
    final position = snapshot.data ?? Duration.zero;
    final duration = player.duration ?? Duration.zero;

    assert(direction == "fwd" || direction == "bwd");

    switch (direction) {
      case "fwd":
        Widget button = IconButton(
          onPressed: () {
            Duration newTime = (position + Duration(seconds: 30));
            newTime = Duration(
              seconds: min(duration.inSeconds, newTime.inSeconds),
            );
            player.seek(newTime);
          },
          icon: Iconify(Tabler["rewind-forward-30"]),
        );
        return button;
      case "bwd":
        Widget button = IconButton(
          onPressed: () {
            Duration newTime = (position - Duration(seconds: 15));
            newTime = Duration(
              seconds: max(Duration.zero.inSeconds, newTime.inSeconds),
            );
            player.seek(newTime);
          },
          icon: Iconify(Tabler["rewind-backward-15"]),
        );
        return button;
      default:
        return Container();
    }
  }

  return builder;
}

AsyncWidgetBuilder<PlayerState> _playButton(AudioPlayer player) {
  Widget builder(BuildContext context, AsyncSnapshot<PlayerState> snapshot) {
    final playerState = snapshot.data;
    final status = playerState?.processingState;
    final playing = playerState?.playing;

    if (status == ProcessingState.loading ||
        status == ProcessingState.buffering) {
      return CircularProgressIndicator();
    }

    if (playing != true) {
      return IconButton(
        onPressed: player.play,
        icon: Iconify(Tabler["player-play-filled"]),
      );
    }

    if (status != ProcessingState.completed) {
      return IconButton(
        onPressed: player.pause,
        icon: Iconify(Tabler["player-pause-filled"]),
      );
    }

    return IconButton(
      onPressed: () => player.seek(Duration.zero),
      icon: Iconify(Tabler["reload"]),
    );
  }

  return builder;
}

AsyncWidgetBuilder<PlayerState> _playbackSpeedButton(AudioPlayer player) {
  Widget builder(BuildContext context, AsyncSnapshot<PlayerState> snapshot) {
    var speeds =
        [0.5, 0.75, 1.0, 1.25, 1.50, 2.0].map((speed) {
          return DropdownMenuEntry(value: speed, label: "$speed");
        }).toList();
    return DropdownMenu<double>(
      initialSelection: 1.0,
      trailingIcon: Iconify(Tabler["arrow-forward-up-double"]),
      // label: const Text("Speed"),
      dropdownMenuEntries: speeds,
      inputDecorationTheme: const InputDecorationTheme(
        filled: true,
        contentPadding: EdgeInsets.only(left: 16, right: 16),
        border: InputBorder.none,
      ),
      onSelected: (speed) {
        player.setSpeed(speed ?? 1.0);
      },
    );
  }

  return builder;
}

Widget time(Duration duration) {
  final printed = durationUtils.prettyPrintDuration(duration);
  return Text(printed, textScaler: TextScaler.linear(0.875), style: TextStyle(fontWeight: FontWeight.w600),);
}

AsyncWidgetBuilder<Duration> _nowTime(AudioPlayer player) {
  Widget builder(BuildContext context, AsyncSnapshot<Duration> snapshot) {
    final position = snapshot.data ?? Duration.zero;
    return time(position);
  }

  return builder;
}

AsyncWidgetBuilder<Duration> _maxTime(AudioPlayer player) {
  Widget builder(BuildContext context, AsyncSnapshot<Duration> snapshot) {
    final duration = player.duration ?? Duration.zero;
    return time(duration);
  }

  return builder;
}

AsyncWidgetBuilder<Duration> _playbackProgress(AudioPlayer player) {
  Widget builder(BuildContext context, AsyncSnapshot<Duration> snapshot) {
    final position = snapshot.data ?? Duration.zero;
    final duration = player.duration ?? Duration.zero;

    return Slider(
      min: 0.0,
      max: duration.inMilliseconds.toDouble(),
      value: position.inMilliseconds.toDouble(),
      label: durationUtils.prettyPrintDuration(position),
      onChanged: (v) => player.seek(Duration(milliseconds: v.toInt())),
    );
  }

  return builder;
}
