#include <cmath>

#include "Modules/Configuration/Configuration.h"
#include "NeuralNetwork.hpp"

#include "print.h"


NeuralNetwork::NeuralNetwork(Configuration& cfg, std::string filename, int activationFunction)
  : activation_function_(activationFunction)
{

  // Read config from json file
  const std::string mount = "MachineLearning.NeuralNetwork";
  cfg.mount(mount, filename, ConfigurationType::HEAD);
  number_of_input_neurons_ = cfg.get(mount, "number_of_input_neurons").asInt();
  number_of_hidden_neurons_ = cfg.get(mount, "number_of_hidden_neurons").asInt();
  number_of_output_neurons_ = cfg.get(mount, "number_of_output_neurons").asInt();

  // Initialise neurons
  for (int i = 0; i < number_of_input_neurons_; i++)
  {
    input_neurons_.push_back(0.0f);
  }
  for (int i = 0; i < number_of_hidden_neurons_; i++)
  {
    hidden_neurons_.push_back(0.0f);
  }
  for (int i = 0; i < number_of_output_neurons_; i++)
  {
    output_neurons_.push_back(0.0f);
  }

  // Set bias neurons
  input_neurons_.push_back(-1.0f);
  hidden_neurons_.push_back(-1.0f);

  // Load weights from file
  std::vector<double> currentRow;
  Uni::Value weights_row = cfg.get(mount, "weights_input_hidden");
  Uni::Value weights_col;

  // number_of_input_neurons_ + 1 because of the bias neuron!
  for (int i = 0; i < number_of_input_neurons_ + 1; i++)
  {
    currentRow.clear();
    weights_col = weights_row[i];
    for (int j = 0; j < number_of_hidden_neurons_; j++)
    {
      currentRow.push_back(weights_col[j].asDouble());
    }

    weights_input_hidden_.push_back(currentRow);
  }

  weights_row = cfg.get(mount, "weights_hidden_output");
  for (int i = 0; i < number_of_hidden_neurons_ + 1; i++)
  {
    currentRow.clear();
    weights_col = weights_row[i];
    for (int j = 0; j < number_of_output_neurons_; j++)
    {
      currentRow.push_back(weights_col[j].asDouble());
    }

    weights_hidden_output_.push_back(currentRow);
  }
}


std::vector<double>& NeuralNetwork::getOutputLayer()
{
  return output_neurons_;
}

std::vector<double>& NeuralNetwork::getHiddenLayer()
{
  return hidden_neurons_;
}

std::vector<double>& NeuralNetwork::getInputLayer()
{
  return input_neurons_;
}

inline void NeuralNetwork::activationFunction(std::vector<double>& input)
{
  switch (activation_function_)
  {
    case 0:
      sigmoid(input);
      break;
    case 1:
      relu(input);
      break;
    default:
      print("Illegal activation function paramter", LogLevel::ERROR);
      break;
  }
}

inline void NeuralNetwork::sigmoid(std::vector<double>& input)
{
  for (auto& it : input)
  {
    it = 1 / (1 + exp(-(it)));
  }
}

inline void NeuralNetwork::relu(std::vector<double>& input)
{
  for (auto& it : input)
  {
    it = (it < 0) ? 0 : it;
  }
}

void NeuralNetwork::feedForward(std::vector<double>& input)
{
  int i = 0;
  int j = 0;
  for (auto it = input_neurons_.begin(); it != input_neurons_.end(); it++, i++)
  {
    if (i < number_of_input_neurons_)
      *it = input[i];
  }
  i = 0;
  for (auto itHidden = hidden_neurons_.begin(); itHidden != hidden_neurons_.end(); itHidden++, i++)
  {
    if (i < number_of_hidden_neurons_)
      *itHidden = 0;
    j = 0;
    for (auto itInput = input_neurons_.begin(); itInput != input_neurons_.end(); itInput++, j++)
    {
      *itHidden += *itInput * weights_input_hidden_[j][i];
    }
  }
  activationFunction(hidden_neurons_);
  i = 0;
  for (auto itOutput = output_neurons_.begin(); itOutput != output_neurons_.end(); itOutput++, i++)
  {
    *itOutput = 0;
    j = 0;
    for (auto itHidden = hidden_neurons_.begin(); itHidden != hidden_neurons_.end(); itHidden++, j++)
    {
      *itOutput += *itHidden * weights_hidden_output_[j][i];
    }
  }
  activationFunction(output_neurons_);
}
