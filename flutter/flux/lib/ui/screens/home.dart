import 'package:flutter/material.dart';

class HomeScreen extends StatefulWidget {
  final List<Widget> screens;
  final List<NavigationDestination Function(BuildContext)> destinations;

  const HomeScreen({
    super.key,
    required this.screens,
    required this.destinations,
  });

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  int _selectedIndex = 0;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: widget.screens[_selectedIndex],
      bottomNavigationBar: NavigationBar(
        selectedIndex: _selectedIndex,
        onDestinationSelected: (index) {
          setState(() {
            _selectedIndex = index;
          });
        },
        destinations:
            widget.destinations
                .map((destination) => destination(context))
                .toList(),
      ),
    );
  }
}
