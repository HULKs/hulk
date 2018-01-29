#pragma once
#include <vector>
#include <string>


class Configuration;

/**
 * @brief The NeuralNetwork class
 * This class provides a simple neural network which consists of one input layer,
 * one hidden layer and one ouput layer. The number of neurons per layer is configurable
 * via a json file. Usage example:
 *
 * NeuralNetwork nn("network.json");
 * nn.feedForward(inputData);
 * std::vector<double> result = nn.getOutputLayer();
 */
class NeuralNetwork {

public:
    /**
     * @brief NeuralNetwork
     * This is the constructor for the Neural Network
     * @param cfg
     * This is the Configuration instance
     * @param filename
     * This is the filename of the json file which contains all the parameters
     * @param activationFunction
     * Choose between different activation functions
     * 0 : Sigmoid Function
     * 1 : Rectified linear unit
     */
    NeuralNetwork(Configuration& cfg, std::string filename, int activationFunction);
    /**
     * @brief feedForward
     * This method makes all the necessary calculations.
     * @param input
     * A vector of type double which will be fed in to the input layer
     */
    void feedForward(std::vector<double>& input);

    /**
     * @brief getInputLayer
     * @return
     * This returns the input layer
     */
    std::vector<double>& getInputLayer();
    /**
     * @brief getHiddenayer
     * @return
     * This returns the hidden layer
     */
    std::vector<double>& getHiddenLayer();
    /**
     * @brief getOutputLayer
     * @return
     * This returns the output layer
     */
    std::vector<double>& getOutputLayer();
private:

    void activationFunction(std::vector<double> &input);

    void sigmoid(std::vector<double>& input);
    void relu(std::vector<double>& input);

    int activation_function_;
    int number_of_input_neurons_;
    int number_of_hidden_neurons_;
    int number_of_output_neurons_;

    std::vector<double> input_neurons_;
    std::vector<double> hidden_neurons_;
    std::vector<double> output_neurons_;

    std::vector<std::vector<double>> weights_input_hidden_;
    std::vector<std::vector<double>> weights_hidden_output_;

};
