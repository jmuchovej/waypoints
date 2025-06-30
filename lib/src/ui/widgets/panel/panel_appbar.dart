import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

class PanelAppBar extends ConsumerStatefulWidget
    implements PreferredSizeWidget {
  const PanelAppBar({
    super.key,
    required this.title,
    this.actions = const [],
    required double size,
    required this.scrollController,
    this.padding = const EdgeInsets.all(8.0),
  }) : _size = size;

  final String title;
  final List<Widget> actions;
  final double _size;
  final EdgeInsetsGeometry padding;
  final ScrollController? scrollController;

  @override
  ConsumerState<ConsumerStatefulWidget> createState() => _PanelHeaderState();

  @override
  // TODO: implement preferredSize
  Size get preferredSize => Size.fromHeight(_size + padding.vertical);

  static double computePreferredSize(BuildContext context) {
    final fontSize = TextTheme.of(context).bodyMedium?.fontSize ?? 12;
    final appBarHeight = fontSize * 4;
    return appBarHeight + 1;
  }
}

class _PanelHeaderState extends ConsumerState<PanelAppBar> {
  bool _showAppBarTitle = false;
  late ScrollController _scrollController;

  @override
  void initState() {
    super.initState();
    _scrollController = widget.scrollController ?? ScrollController();
    _scrollController.addListener(_onScroll);
  }

  @override
  void didUpdateWidget(PanelAppBar oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.scrollController != widget.scrollController) {
      oldWidget.scrollController?.removeListener(_onScroll);
      _scrollController = widget.scrollController ?? ScrollController();
      _scrollController.addListener(_onScroll);
    }
  }

  @override
  void dispose() {
    _scrollController.removeListener(_onScroll);
    _scrollController.dispose();
    super.dispose();
  }

  void _onScroll() {
    final shouldShowTitle = _scrollController.offset > 100;
    if (shouldShowTitle != _showAppBarTitle) {
      setState(() {
        _showAppBarTitle = shouldShowTitle;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    // if (MediaQuery.of(context).size.width <= 768) {
    //   return const SizedBox.shrink();
    // }

    return SafeArea(
      child: Column(
        children: [
          AppBar(
            backgroundColor: ColorScheme.of(context).surfaceContainer,
            // clipBehavior: Clip.none,
            elevation: 0,
            scrolledUnderElevation: 0,
            toolbarHeight: widget.preferredSize.height - 1,
            automaticallyImplyLeading: false,
            title: AnimatedSlide(
              duration: const Duration(milliseconds: 200),
              curve: Curves.easeInOut,
              offset: Offset(0, _showAppBarTitle ? 0 : 1),
              child: AnimatedOpacity(
                duration: const Duration(milliseconds: 200),
                opacity: _showAppBarTitle ? 1 : 0,
                child: Text(widget.title),
              ),
            ),
            actions: widget.actions,
            actionsPadding: widget.padding,
          ),
          AnimatedContainer(
            duration: const Duration(milliseconds: 200),
            height: 1,
            color:
                _showAppBarTitle
                    ? ColorScheme.of(context).outlineVariant
                    : ColorScheme.of(context).surfaceContainer,
          ),
        ],
      ),
    );
  }
}
