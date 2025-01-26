pub trait LossField {
    type Parameter;
    type Gradient;
    type Loss;

    fn loss(&self, parameter: Self::Parameter) -> Self::Loss;
    fn grad(&self, parameter: Self::Parameter) -> Self::Gradient;
}
