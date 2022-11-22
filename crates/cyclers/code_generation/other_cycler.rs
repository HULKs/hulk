pub enum OtherCycler<'a> {
    Consumer {
        cycler_instance_name: &'a str,
        cycler_module_name: &'a str,
    },
    Reader {
        cycler_instance_name: &'a str,
        cycler_module_name: &'a str,
    },
}
