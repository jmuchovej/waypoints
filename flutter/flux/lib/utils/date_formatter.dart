import 'package:flutter/material.dart';
import 'package:intl/intl.dart';

class DateFormatter {
  static final _dateFormat = DateFormat('MM/dd/yyyy');
  static final _dateFormatWithDay = DateFormat('EEE, MMM d, yyyy');

  static String formatDate(DateTime date) {
    return _dateFormat.format(date);
  }

  static String formatDateWithDay(DateTime date) {
    return _dateFormatWithDay.format(date);
  }

  static String formatDueDate(DateTime date) {
    final now = DateTime.now();
    final today = DateTime(now.year, now.month, now.day);
    final tomorrow = today.add(const Duration(days: 1));
    final dateOnly = DateTime(date.year, date.month, date.day);

    if (dateOnly == today) {
      return 'Today';
    } else if (dateOnly == tomorrow) {
      return 'Tomorrow';
    } else if (dateOnly.difference(today).inDays < 7) {
      return DateFormat('EEEE').format(date); // Day of week
    } else {
      return _dateFormatWithDay.format(date);
    }
  }

  static String formatTimeOfDay(TimeOfDay time) {
    final hour = time.hourOfPeriod == 0 ? 12 : time.hourOfPeriod;
    final minute = time.minute.toString().padLeft(2, '0');
    final period = time.period == DayPeriod.am ? 'AM' : 'PM';
    return '$hour:$minute $period';
  }
}
