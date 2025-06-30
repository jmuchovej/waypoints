import 'package:objectbox/objectbox.dart';
import '../../thing/interface.dart';
import '../activity_entity.dart';
import '../../thing/thing_entity.dart';

// Resolver is now auto-registered by ActivitySnapshotRepository (when created)
// No need for manual resolver configuration!

@Entity()
class ActivitySnapshotEntity implements BaseThing {
  ActivitySnapshotEntity({
    required this.uuid,
    required this.timestamp,
    this.latitude,
    this.longitude,
    this.accuracyH,
    this.accuracyV,
    this.locationSource,
    this.satelliteCount,
    this.heading,
    this.speed,
    this.deviceID,
    this.platform,
    List<String>? nearbyNetworks,
    List<String>? nearbyDevices,
    List<String>? activeApplications,
    DateTime? createDate,
    DateTime? modifyDate,
    this.archived = false,
    this.deleted = false,
  }) : createDate = createDate ?? DateTime.now(),
       modifyDate = modifyDate ?? DateTime.now(),
       nearbyNetworks = nearbyNetworks ?? [],
       nearbyDevices = nearbyDevices ?? [],
       activeApplications = activeApplications ?? [];

  static const key = 'activity_snapshot';

  @override
  @Id()
  int id = 0;

  @override
  String uuid;

  @Property(type: PropertyType.dateNano)
  DateTime timestamp;

  // Location data
  double? latitude;
  double? longitude;

  /// Horizontal accuracy (ground position, in meters)
  double? accuracyH;

  /// Vertical accuracy (altitude, in meters)
  double? accuracyV;

  /// Where is this coming from? (e.g., GPS, Wi-Fi, Cellular, fused, etc.)
  String? locationSource;

  /// How many GPS satellites were used? (if available)
  int? satelliteCount;
  double? speed;
  double? heading;

  // Device context
  String? deviceID;
  String? platform;
  List<String> nearbyNetworks;
  List<String> nearbyDevices;

  // App usage data
  List<String> activeApplications;

  final activity = ToMany<ActivityEntity>();

  @override
  String get type => key;

  @override
  bool archived;

  @override
  final ToMany<ThingEntity> children = ToMany<ThingEntity>();

  @Property(type: PropertyType.dateNano)
  @override
  DateTime createDate;

  @override
  bool deleted;

  @Property(type: PropertyType.dateNano)
  @override
  DateTime modifyDate;

  /// Direct reference to the ThingEntity registration
  /// This creates a 1:1 relationship between this ActivitySnapshot and its registry entry
  @override
  final ToOne<ThingEntity> thing = ToOne<ThingEntity>();

  @override
  String? getSearchableText() => null;
}
