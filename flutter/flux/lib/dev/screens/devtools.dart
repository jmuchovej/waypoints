// lib/ui/screens/dev_tools.dart
import 'package:flutter/material.dart';
import 'package:flux/dev/database/seed.dart';
import 'package:flux/objectbox.dart';
import 'package:path_provider/path_provider.dart';

class DevToolsScreen extends StatefulWidget {
  @override
  _DevToolsScreenState createState() => _DevToolsScreenState();
}

class _DevToolsScreenState extends State<DevToolsScreen> {
  int _taskCount = 0;
  int _projectCount = 0;
  int _activityCount = 0;
  bool _isLoading = false;

  @override
  void initState() {
    super.initState();
    _refreshCounts();
  }

  Future<void> _refreshCounts() async {
    setState(() {
      _isLoading = true;
    });

    final taskCount = ObjectBox.taskBox().count();
    final projectCount = ObjectBox.taskListBox().count();
    final activityCount = ObjectBox.activityBox().count();

    setState(() {
      _taskCount = taskCount;
      _projectCount = projectCount;
      _activityCount = activityCount;
      _isLoading = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text('Developer Tools'),
        actions: [
          IconButton(icon: Icon(Icons.refresh), onPressed: _refreshCounts),
        ],
      ),
      body:
          _isLoading
              ? Center(child: CircularProgressIndicator())
              : SingleChildScrollView(
                padding: EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    _buildDatabaseStatsCard(),
                    SizedBox(height: 24),
                    _buildDataControls(),
                    SizedBox(height: 24),
                    _buildDeveloperOptions(),
                  ],
                ),
              ),
    );
  }

  Widget _buildDatabaseStatsCard() {
    return Card(
      child: Padding(
        padding: EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Database Statistics',
              style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
            ),
            SizedBox(height: 16),
            _buildStatRow('Tasks', _taskCount),
            _buildStatRow('Projects', _projectCount),
            _buildStatRow('Activities', _activityCount),
            _buildStatRow(
              'Total Items',
              _taskCount + _projectCount + _activityCount,
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildStatRow(String label, int count) {
    return Padding(
      padding: EdgeInsets.symmetric(vertical: 4),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label),
          Text(count.toString(), style: TextStyle(fontWeight: FontWeight.bold)),
        ],
      ),
    );
  }

  Widget _buildDataControls() {
    return Card(
      child: Padding(
        padding: EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Sample Data Controls',
              style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
            ),
            SizedBox(height: 16),
            Row(
              children: [
                Expanded(
                  child: ElevatedButton.icon(
                    icon: Icon(Icons.data_array),
                    label: Text('Generate Sample Data'),
                    onPressed: () async {
                      await SeedDataService.seedIfNeeded(force: true);
                      _refreshCounts();
                      ScaffoldMessenger.of(context).showSnackBar(
                        SnackBar(content: Text('Sample data generated')),
                      );
                    },
                  ),
                ),
                SizedBox(width: 16),
                Expanded(
                  child: ElevatedButton.icon(
                    icon: Icon(Icons.delete_forever),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: Colors.red,
                      foregroundColor: Colors.white,
                    ),
                    label: Text('Clear All Data'),
                    onPressed: () async {
                      final confirmed = await _showConfirmationDialog();
                      if (confirmed) {
                        await SeedDataService.clearAllData();
                        _refreshCounts();
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(content: Text('All data cleared')),
                        );
                      }
                    },
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildDeveloperOptions() {
    return Card(
      child: Padding(
        padding: EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Developer Options',
              style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
            ),
            SizedBox(height: 16),
            ListTile(
              title: Text('View Database Location'),
              subtitle: Text('See where your data is stored'),
              trailing: Icon(Icons.folder),
              onTap: () async {
                final directory = await getApplicationDocumentsDirectory();
                final dbPath = '${directory.path}/obx';

                showDialog(
                  context: context,
                  builder:
                      (context) => AlertDialog(
                        title: Text('Database Location'),
                        content: SelectableText(dbPath),
                        actions: [
                          TextButton(
                            onPressed: () => Navigator.pop(context),
                            child: Text('OK'),
                          ),
                        ],
                      ),
                );
              },
            ),
            Divider(),
            ListTile(
              title: Text('Export Database'),
              subtitle: Text('Not implemented yet'),
              trailing: Icon(Icons.download),
              enabled: false,
              onTap: null,
            ),
            Divider(),
            ListTile(
              title: Text('View Logs'),
              subtitle: Text('Not implemented yet'),
              trailing: Icon(Icons.list_alt),
              enabled: false,
              onTap: null,
            ),
          ],
        ),
      ),
    );
  }

  Future<bool> _showConfirmationDialog() async {
    return await showDialog<bool>(
          context: context,
          builder:
              (context) => AlertDialog(
                title: Text('Confirm Clear Data'),
                content: Text(
                  'This will delete ALL data in the database, including tasks, projects, and activities. '
                  'This action cannot be undone.\n\nAre you sure you want to proceed?',
                ),
                actions: [
                  TextButton(
                    onPressed: () => Navigator.pop(context, false),
                    child: Text('Cancel'),
                  ),
                  TextButton(
                    style: TextButton.styleFrom(foregroundColor: Colors.red),
                    onPressed: () => Navigator.pop(context, true),
                    child: Text('Clear All Data'),
                  ),
                ],
              ),
        ) ??
        false;
  }
}
