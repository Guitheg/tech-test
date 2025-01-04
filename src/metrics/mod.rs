pub(crate) mod twap;
pub(crate) mod storage;


pub(crate) trait Metric<MetricType, InputType> {
    fn update(&mut self, new_value: InputType) -> Result<Option<MetricType>, String>;
    fn current(&self) -> MetricType;
}