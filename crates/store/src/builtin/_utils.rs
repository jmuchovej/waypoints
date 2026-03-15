use domain::{
    Color, Feature, FeatureKind, Icon, Reference, Select, SelectBuilder, SelectGroup, SelectOption,
};

pub fn status_multi_select() -> Select {
    let status_awaiting_group = {
        let sometime = SelectOption::builder()
            .with_name("Sometime")
            .with_icon(Icon::system_emoji("melting-face"))
            .with_color(Color::Default)
            .build();
        let not_started = SelectOption::builder()
            .with_name("Not Started")
            .with_icon(Icon::system_emoji("alarm-clock"))
            .with_color(Color::Gray)
            .build();
        let id = not_started.id.clone();
        SelectGroup::builder()
            .with_name("Pending")
            .with_option(sometime)
            .with_option(not_started)
            .with_default_option(id)
            .with_single_selection()
            .build()
    };

    let status_progress_group = {
        let waiting = SelectOption::builder()
            .with_name("Waiting")
            .with_icon(Icon::system_emoji("vertical-traffic-light"))
            .with_color(Color::Blue)
            .build();
        let in_progress = SelectOption::builder()
            .with_name("In Progress")
            .with_icon(Icon::system_emoji("construction"))
            .with_color(Color::Yellow)
            .build();
        let id = in_progress.id.clone();
        SelectGroup::builder()
            .with_name("In Progress")
            .with_option(waiting)
            .with_option(in_progress)
            .with_default_option(id)
            .with_single_selection()
            .build()
    };

    let status_finished_group = {
        let completed = SelectOption::builder()
            .with_name("Completed")
            .with_icon(Icon::system_emoji("party-popper"))
            .with_color(Color::Green)
            .build();
        let id = completed.id.clone();
        let dropped = SelectOption::builder()
            .with_name("Dropped")
            .with_icon(Icon::system_emoji("drop-of-blood"))
            .with_color(Color::Red)
            .build();
        let cancelled = SelectOption::builder()
            .with_name("Cancelled")
            .with_icon(Icon::system_emoji("person-gesturing-no"))
            .with_color(Color::Orange)
            .build();
        SelectGroup::builder()
            .with_name("Finished")
            .with_option(completed)
            .with_option(dropped)
            .with_option(cancelled)
            .with_default_option(id)
            .with_single_selection()
            .build()
    };

    Select::builder()
        .with_group(status_awaiting_group, true.into())
        .with_group(status_progress_group, None)
        .with_group(status_finished_group, None)
        .with_single_selection()
        .build()
}

pub fn tags_feature() -> Feature {
    Feature::builder()
        .with_name("Tags")
        .with_kind(Reference::builder().with_display_mention().build().into())
        .build()
}
