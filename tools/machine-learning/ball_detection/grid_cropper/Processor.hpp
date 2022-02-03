#pragma once

#include <CompiledNN/CompiledNN.h>
#include <CompiledNN/Model.h>
#include <Tensor.h>
#include <filesystem>
#include <runner.hpp>
#include <unordered_map>
#include <vector>

#include "Circle.hpp"
#include "CircleWithMetadata.hpp"
#include "Cluster.hpp"
#include "Configuration.hpp"
#include "EvaluationResult.hpp"
#include "Image.hpp"

namespace Hulks::GridCropper
{

  class Processor
  {
  public:
    using ItemType = std::filesystem::path;
    static std::vector<ItemType> prologue(Hulks::Runner::Runner<Processor>& runner,
                                          const Configuration& configuration);
    static void epilogue(Hulks::Runner::Runner<Processor>& runner,
                         const Configuration& configuration);
    Processor(Hulks::Runner::Runner<Processor>& runner, const Configuration& configuration);
    void process(const ItemType& imagePath);

  private:
    void generateCandidates();
    EvaluationResult evaluateCandidate(const Circle<float>& candidate);
    bool copySampleToTensor(const Circle<float>& candidate, NeuralNetwork::TensorXf& tensor) const;
    static float intersectionRatio(float sampleXLeft, float correctedXLeft, float sampleXRight,
                                   float correctedXRight, float sampleYTop, float correctedYTop,
                                   float sampleYBottom, float correctedYBottom);
    static float circleIntersectionRatio(const Circle<float>& correctedCircle,
                                         const Circle<float>& sampleCircle);
    static float imageIntersectionRatio(const Circle<float>& sampleCircle, const Image& image);
    void clusterCandidates();

    Hulks::Runner::Runner<Processor>& runner_;
    const Configuration& configuration_;

    NeuralNetwork::CompiledNN classifierCompiler_{};
    NeuralNetwork::CompiledNN positionerCompiler_{};
    Image image_{};
    std::size_t amountOfCandidates_{0};
    std::vector<CircleWithMetadata> acceptedCandidates_{};
    std::vector<Cluster> clusteredAcceptedCandidates_{};

    static std::filesystem::path annotationsFileDirectory__;
    static std::unordered_map<ItemType, std::vector<Circle<float>>> annotations__;
  };

} // namespace Hulks::GridCropper
