#include <CompiledNN.h>
#include <cxxopts.hpp>
#include <iostream>
#include <nlohmann/json.hpp>

nlohmann::json predictModel(nlohmann::json input, const std::string& modelPath)
{
  ASSERT(input.is_array());
  ASSERT(input.size() > 0);

  const auto isFloat = input[0].is_number_float();
  for (const auto& value : input)
  {
    ASSERT(value.is_number());
    ASSERT(isFloat == value.is_number_float());
  }

  NeuralNetwork::Model model;
  model.load(modelPath);

  if (!isFloat)
  {
    model.setInputUInt8(0);
  }

  NeuralNetwork::CompiledNN neuralNetwork;
  neuralNetwork.compile(model);

  for (unsigned int i = 0; i < input.size(); ++i)
  {
    if (isFloat)
    {
      neuralNetwork.input(0)[i] = input[i].get<float>();
    }
    else
    {
      neuralNetwork.input(0)[i] = input[i].get<uint8_t>();
    }
  }

  neuralNetwork.apply();

  nlohmann::json output;
  for (const auto& value : neuralNetwork.output(0))
  {
    output.push_back(value);
  }

  return output;
}

int main(int argc, char* argv[])
{
  cxxopts::Options options{
      "predicter", "Predicts a given model with given JSON input array (JSON lines of array "
                   "of float/int) and prints JSON output (JSON lines of array of float)."};

  options.add_option("", "h", "help", "Print help", cxxopts::value<bool>(), "");

  std::string modelPath;
  options.add_option("", "", "MODEL_PATH",
                     "The directories containing images or single image files",
                     cxxopts::value<decltype(modelPath)>(modelPath), "");
  options.parse_positional({"MODEL_PATH"});
  options.positional_help("MODEL_PATH");

  const auto result = options.parse(argc, argv);

  if (result.count("help") != 0)
  {
    std::cout << options.help() << '\n';
    return 0;
  }

  for (std::string line; std::getline(std::cin, line);)
  {
    std::cout << predictModel(nlohmann::json::parse(line), modelPath).dump() << '\n';
  }

  return 0;
}
