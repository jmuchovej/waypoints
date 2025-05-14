String prettyPrintDuration(Duration duration) {
  String twoDigits(int n) => n.toString().padLeft(2, "0");
  String timeMM = twoDigits(duration.inMinutes.remainder(60).abs());
  String timeSS = twoDigits(duration.inSeconds.remainder(60).abs());
  return "$timeMM:$timeSS";
}
