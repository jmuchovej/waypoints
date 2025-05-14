import "package:flutter/material.dart";
import "package:just_audio/just_audio.dart";
import "package:transcription/service.dart";
import "package:scrollable_positioned_list/scrollable_positioned_list.dart";

class TranscriptWidget extends StatefulWidget {
  final TranscriptJSON transcript;
  final AudioPlayer player;

  const TranscriptWidget({
    super.key,
    required this.transcript,
    required this.player,
  });

  @override
  TranscriptState createState() => TranscriptState();
}

class TranscriptState extends State<TranscriptWidget> {
  int _scrollIndex = 0;
  final ItemScrollController _itemScrollController = ItemScrollController();
  final ScrollOffsetController _scrollOffsetController =
      ScrollOffsetController();
  final ItemPositionsListener _itemPositionsListener =
      ItemPositionsListener.create();
  final ScrollOffsetListener _scrollOffsetListener =
      ScrollOffsetListener.create();
  final ValueNotifier<Duration> _notifier = ValueNotifier(Duration.zero);

  @override
  void initState() {
    super.initState();
    widget.player.positionStream.listen(_sync);
    widget.player.positionStream.listen((position) {
      _notifier.value = position;
    });
  }

  void _sync(Duration position) {
    for (int index = 0; index < widget.transcript.length; index++) {
      final utterance = widget.transcript[index];
      if (position.inMilliseconds >= utterance["start"] &&
          position.inMilliseconds <= utterance["end"]) {
        if (_scrollIndex != index) {
          _scrollIndex = index;
          _itemScrollController.scrollTo(
            index: index,
            curve: Curves.easeInOut,
            duration: Duration(milliseconds: 500),
          );
        }
        break;
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<Duration>(
      valueListenable: _notifier,
      builder: (context, position, _) {
        return ScrollablePositionedList.builder(
          itemCount: widget.transcript.length,
          itemScrollController: _itemScrollController,
          scrollOffsetController: _scrollOffsetController,
          itemPositionsListener: _itemPositionsListener,
          scrollOffsetListener: _scrollOffsetListener,
          itemBuilder: (context, index) {
            final segment = widget.transcript[index];
            return _utteranceBuilder(segment, position);
          },
        );
      },
    );
  }

  Widget _utteranceBuilder(Map<String, dynamic> utterance, Duration position) {
    Widget speaker = Text(
      "${utterance["speaker"]}",
      style: TextStyle(fontWeight: FontWeight.bold),
    );

    String prettyPrintDuration(Duration duration) {
      String twoDigits(int n) => n.toString().padLeft(2, "0");
      String timeMM = twoDigits(duration.inMinutes.remainder(60).abs());
      String timeSS = twoDigits(duration.inSeconds.remainder(60).abs());
      return "$timeMM:$timeSS";
    }

    Widget startTime = Text(
      prettyPrintDuration(Duration(milliseconds: utterance["start"])),
      style: TextStyle(fontWeight: FontWeight.bold),
    );
    return Container(
      padding: EdgeInsets.symmetric(vertical: 8.0, horizontal: 16.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [speaker, startTime],
          ),
          RichText(
            text: TextSpan(
              children:
                  utterance["words"].map<TextSpan>((word) {
                    Duration msBeg = Duration(milliseconds: word["start"]);
                    Duration msEnd = Duration(milliseconds: word["end"]);
                    bool active = (msBeg <= position && position <= msEnd);

                    TextStyle style = TextStyle(
                      backgroundColor:
                          active ? Colors.amber : Colors.transparent,
                    );

                    return TextSpan(
                      style: TextStyle(height: 1.4),
                      children: [
                        TextSpan(text: "${word["text"]}", style: style),
                        TextSpan(
                          text: " ",
                          style: TextStyle(backgroundColor: Colors.transparent),
                        ),
                      ],
                    );
                  }).toList(),
              style: TextStyle(color: Colors.blueGrey[800]),
            ),
          ),
        ],
      ),
    );
  }
}
