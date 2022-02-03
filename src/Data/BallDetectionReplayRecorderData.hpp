#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Circle.hpp"

struct BallDetectionData : public Uni::From, public Uni::To
{
  struct CandidateCircle : public Uni::From, public Uni::To
  {
    CandidateCircle() = default;

    CandidateCircle(float preClassifierConfidence, float ballConfidence, Circle<int> circle)
      : preClassifierConfidence{preClassifierConfidence}
      , ballConfidence{ballConfidence}
      , circle{std::move(circle)}
    {
    }

    /// the pre-classifier confidence the ball detection gave this candidate circle
    float preClassifierConfidence{0.f};
    /// the confidence the ball detection gave this candidate circle
    float ballConfidence{0.f};
    /// the 444 candidate circle
    Circle<int> circle{};

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["preClassifierConfidence"] << preClassifierConfidence;
      value["ballConfidence"] << ballConfidence;
      value["circle"] << circle;
    }

    void fromValue(const Uni::Value& value) override
    {
      value["preClassifierConfidence"] >> preClassifierConfidence;
      value["ballConfidence"] >> ballConfidence;
      value["circle"] >> circle;
    }
  };

  /// candidates of the last frame
  std::vector<CandidateCircle> lastCandidates;
  /// candidates of the current frame
  std::vector<CandidateCircle> candidates;

  struct Cluster : public Uni::From, public Uni::To
  {
    /// original candidate circle with corrected circle
    struct Candidate : public Uni::From, public Uni::To
    {
      Circle<int> candidateCircle;
      Circle<float> correctedCircle;

      Candidate() = default;

      Candidate(Circle<int> candidateCircle, Circle<float> correctedCircle)
        : candidateCircle{std::move(candidateCircle)}
        , correctedCircle{std::move(correctedCircle)}
      {
      }

      void toValue(Uni::Value& value) const override
      {
        value = Uni::Value(Uni::ValueType::OBJECT);
        value["candidateCircle"] << candidateCircle;
        value["correctedCircle"] << correctedCircle;
      }

      void fromValue(const Uni::Value& value) override
      {
        value["candidateCircle"] >> candidateCircle;
        value["correctedCircle"] >> correctedCircle;
      }
    };

    /// merged circle of the cluster
    Circle<float> mergedCircle;
    /// all candidates belonging to cluster
    std::vector<Candidate> candidatesInCluster;

    Cluster() = default;

    Cluster(Circle<float> mergedCircle, std::vector<Candidate> candidatesInCluster)
      : mergedCircle{std::move(mergedCircle)}
      , candidatesInCluster{std::move(candidatesInCluster)}
    {
    }

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["mergedCircle"] << mergedCircle;
      value["candidatesInCluster"] << candidatesInCluster;
    }

    void fromValue(const Uni::Value& value) override
    {
      value["mergedCircle"] >> mergedCircle;
      value["candidatesInCluster"] >> candidatesInCluster;
    }
  };

  /// cluster of accepted candidates
  std::vector<Cluster> clusters;

  void reset()
  {
    lastCandidates.clear();
    candidates.clear();
    clusters.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["lastCandidates"] << lastCandidates;
    value["candidates"] << candidates;
    value["clusters"] << clusters;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["lastCandidates"] >> lastCandidates;
    value["candidates"] >> candidates;
    value["clusters"] >> clusters;
  }
};

class BallDetectionReplayRecorderData : public DataType<BallDetectionReplayRecorderData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"BallDetectionReplayRecorderData"};

  /// whether the current cycle should be recorded
  bool recordingRequested = false;

  BallDetectionData data;

  void reset() override
  {
    recordingRequested = false;
    data.reset();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["recordingRequested"] << recordingRequested;
    value["data"] << data;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["recordingRequested"] >> recordingRequested;
    value["data"] >> data;
  }
};
