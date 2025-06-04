import 'package:follow_the_leader/follow_the_leader.dart';

class LeaderLinkManager {
  final _links = <String, LeaderLink>{};
  final _linkListeners = <String, int>{};

  LeaderLink getLink(String nodeID) {
    return _links.putIfAbsent(nodeID, () => LeaderLink());
  }

  void retainLink(String nodeId) {
    _linkListeners[nodeId] = (_linkListeners[nodeId] ?? 0) + 1;
  }

  void releaseLink(String nodeId) {
    final count = (_linkListeners[nodeId] ?? 1) - 1;
    if (count <= 0) {
      _linkListeners.remove(nodeId);
      // Optionally remove unused links after a delay
      Future.delayed(Duration(seconds: 5), () {
        if (!_linkListeners.containsKey(nodeId)) {
          _links.remove(nodeId);
        }
      });
    } else {
      _linkListeners[nodeId] = count;
    }
  }
}
