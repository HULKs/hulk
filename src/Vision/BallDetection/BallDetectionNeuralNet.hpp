#pragma once

#include "Data/BallState.hpp"
#include "Data/CycleInfo.hpp"
#include <CompiledNN/CompiledNN.h>
#include <CompiledNN/Model.h>
#include <array>
#include <mutex>
#include <vector>

#include "Data/BallData.hpp"
#include "Data/BallDetectionReplayRecorderData.hpp"
#include "Data/CameraMatrix.hpp"
#include "Data/FieldBorder.hpp"
#include "Data/FieldColor.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/ImageData.hpp"
#include "Data/ImageSegments.hpp"
#include "Data/PerspectiveGridCandidates.hpp"
#include "Framework/Module.hpp"
#include "Tools/Math/Circle.hpp"
#include "Tools/Math/Eigen.hpp"

class Brain;

class BallDetectionNeuralNet : public Module<BallDetectionNeuralNet, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"BallDetectionNeuralNet"};
  /**
   * @brief BallDetectionNeuralNet initializes members
   * @param manager a reference to brain
   */
  BallDetectionNeuralNet(const ModuleManagerInterface& manager);

  /**
   * @brief cycle tries to find a ball
   */
  void cycle() override;

private:
  const Dependency<PerspectiveGridCandidates> perspectiveGridCandidates_;
  const Dependency<CameraMatrix> cameraMatrix_;
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<ImageData> imageData_;

  const Parameter<float> mergeRadiusFactor_;
  const Parameter<float> confidenceThresholdPreClassifier_;
  const Parameter<float> confidenceThresholdPreClassifierDebug_;
  const Parameter<float> confidenceThresholdClassifier_;
  const Parameter<float> confidenceThresholdClassifierDebug_;
  const Parameter<float> confidenceFactorWeight_;
  const Parameter<float> correctionProximityFactorWeight_;
  const Parameter<float> imageContainmentFactorWeight_;
  const Parameter<bool> recordAllPositives_;
  const Parameter<bool> recordIfNumberOfPositivesIncreases_;
  const Parameter<bool> recordIfNumberOfPositivesDecreases_;
  const Parameter<bool> drawPreCandidateOutlines_;
  const Parameter<bool> drawPreCandidateAnnotations_;
  const Parameter<bool> drawDebugCandidateOutlines_;
  const Parameter<bool> drawDebugCandidateAnnotations_;
  const Parameter<bool> drawCandidateOutlines_;
  const Parameter<bool> drawCandidateAnnotations_;
  const Parameter<bool> drawDebugBallOutlines_;
  const Parameter<bool> drawDebugBallAnnotations_;
  const Parameter<bool> drawBallOutlines_;
  const Parameter<bool> drawBallAnnotations_;
  const Parameter<bool> drawClusteringAnnotations_;
  /// the edge length in pixel of one sample square
  const Parameter<unsigned int> sampleSize_;
  /// the factor the projected ball size is multiplied to get the actual size for a sample
  const Parameter<float> ballRadiusIncreaseFactor_;
  /// the path to the preclassifier model
  const Parameter<std::string> preclassifierPath_;
  /// the path to the classifier model
  const Parameter<std::string> classifierPath_;
  /// the path to the positioner model
  const Parameter<std::string> positionerPath_;

  /// representation of compilation settings for CompiledNN
  struct CompilationSettings : public Uni::From, public Uni::To
  {
    // CPU features for CompiledNN
    /// use x64 features (additional XMM registers)
    bool useX64 = false;
    /// use SSE features up to 4.2 as supported by NAO V6 (else SSSE3 is used as the max version)
    bool useSSE42 = false;
    /// use AVX and AVX2 features (not supported by NAOs)
    bool useAVX2 = false;

    // Optimizations for CompiledNN
    /// use a less accurate but faster approximation of sigmoid
    bool useExponentialApproximationInSigmoid = false;
    /// use a less accurate but faster approximation of tanh
    bool useExponentialApproximationInTanh = false;

    /**
     * @brief toCompilationSettings returns a pre-setup instance of NeuralNet::CompilationSettings
     * @return the assembled settings
     */
    NeuralNetwork::CompilationSettings toCompilationSettings() const;

    void toValue(Uni::Value& value) const override;
    void fromValue(const Uni::Value& value) override;
  };

  /// Compilation settings for CompiledNN for the preclassifier model
  const Parameter<CompilationSettings> preclassifierCompilationSettings_;
  /// Compilation settings for CompiledNN for the classifier model
  const Parameter<CompilationSettings> classifierCompilationSettings_;
  /// Compilation settings for CompiledNN for the positioner model
  const Parameter<CompilationSettings> positionerCompilationSettings_;

  std::string debugImageMount_;

  // mutex for protecting CompiledNN compilers
  std::mutex compilerMutex_;

  // CompiledNN compilers for inference
  NeuralNetwork::CompiledNN preclassifierCompiler_;
  NeuralNetwork::CompiledNN classifierCompiler_;
  NeuralNetwork::CompiledNN positionerCompiler_;

  /// stores all metadata associated with a candidate
  struct CandidateMetadata
  {
    /// raw circle from the candidate generator
    Circle<int> candidateCircle;
    /// actual size of candidate sample (used in neural networks)
    float sizeInImage444{0.f};
    /// scale factor from candidate coordinates to 444 coordinates
    float scale444{0.f};
    /// confidence of the pre-classifier
    float preClassifierConfidence{0.f};
    /// confidence of the classifier
    float classifierConfidence{0.f};
    /// X-position in candidate coordinates
    float positionX{0.f};
    /// Y-position in candidate coordinates
    float positionY{0.f};
    /// radius in candidate coordinates
    float radius{0.f};
    /// position-corrected circle
    Circle<float> correctedCircle;
  };

  /// contains all candidates with associated metadata
  std::vector<CandidateMetadata> candidates_;

  /// contains the current merged circle of the cluster and all items belonging to cluster
  struct Cluster
  {
    Circle<float> mergedCircle;
    std::vector<std::vector<CandidateMetadata>::const_iterator> candidatesInCluster;

    Cluster(Circle<float> mergedCircle,
            std::vector<std::vector<CandidateMetadata>::const_iterator> candidatesInCluster);
  };
  /// contains clustered accepted candidates
  std::vector<Cluster> clusters_;

  /// the debug strings generated while clustering
  std::string debugStringsOfClustering_;

  /// replay recorder frame data from the last cycle of the top camera
  std::vector<BallDetectionData::CandidateCircle> lastCandidatesTop_;
  /// replay recorder frame data from the last cycle of the bottom camera
  std::vector<BallDetectionData::CandidateCircle> lastCandidatesBottom_;
  /// number of positive candidates from the last cycle of the top camera
  int numberOfLastPositivesTop_;
  /// number of positive candidates from the last cycle of the bottom camera
  int numberOfLastPositivesBottom_;

  /**
   * @brief loads the frozen neural network specified in networkPath_
   */
  void loadNeuralNetwork();

  /**
   * @brief evaluates all candidates
   */
  void evaluateCandidates();

  /**
   * @brief evaluates pre-classifier on candidate and stores results in candidate metadata
   * @param candidate the candidate to evaluate
   */
  void evaluatePreClassifier(CandidateMetadata& candidate);

  /**
   * @brief evaluates classifier on candidate and stores results in candidate metadata
   * @param candidate the candidate to evaluate
   */
  void evaluateClassifier(CandidateMetadata& candidate);

  /**
   * @brief evaluates positioner on candidate and stores results in candidate metadata (this
   * function requires evaluateClassifier() to be executed before)
   * @param candidate the candidate to evaluate
   */
  void evaluatePositioner(CandidateMetadata& candidate);

  /**
   * Samples the image representing the given candidate. This will serve as input for the neural
   * network.
   * @param candidate The candidate to sample an image from
   * @param sampledPatch out: A reference to the sampled image tensor
   */
  void sampleBoundingBox(const CandidateMetadata& candidate,
                         NeuralNetwork::TensorXf& sampledPatch) const;

  /**
   * @brief updates ReplayRecorder data
   */
  void updateReplayRecorderData();

  /**
   * @brief sends the debug image showing candidates and accepted/rejected balls
   */
  void sendDebugImage() const;

  /**
   * @brief clusters all candidates
   */
  void clusterCandidates();

  /**
   * @brief calculates ratio of intersection of corrected in sample
   * @param sampleXLeft left X-coordinate of sample
   * @param correctedXLeft left X-coordinate of corrected
   * @param sampleXRight right X-coordinate of sample
   * @param correctedXRight right X-coordinate of corrected
   * @param sampleYTop top Y-coordinate of sample
   * @param correctedYTop top Y-coordinate of corrected
   * @param sampleYBottom bottom Y-coordinate of sample
   * @param correctedYBottom bottom Y-coordinate of corrected
   * @return intersection ratio (0 = no intersection, 1 = full intersection)
   */
  static float intersectionRatio(float sampleXLeft, float correctedXLeft, float sampleXRight,
                                 float correctedXRight, float sampleYTop, float correctedYTop,
                                 float sampleYBottom, float correctedYBottom);

  /**
   * @brief calculates ratio of intersection of corrected circle in sample circle (as rectangles)
   * @param correctedCircle circle of corrected (interpreted as rectangle)
   * @param sampleCircle circle of sample (interpreted as rectangle)
   * @return intersection ratio (0 = no intersection, 1 = full intersection)
   */
  static float circleIntersectionRatio(const Circle<float>& correctedCircle,
                                       Circle<float> sampleCircle);

  /**
   * @brief calculates ratio of intersection of sample circle in image (as rectangles)
   * @param sampleCircle circle of sample (interpreted as rectangle)
   * @param imageSize size of image
   * @return intersection ratio (0 = no intersection, 1 = full intersection)
   */
  static float imageIntersectionRatio(Circle<float> sampleCircle, const Vector2i& imageSize);

  /// the generated ball
  Production<BallData> ballData_;
  /// the data for ReplayRecorder
  Production<BallDetectionReplayRecorderData> ballDetectionReplayRecorderData_;
};
