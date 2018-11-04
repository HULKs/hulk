#pragma once

#define SIGN(x) ((x > 0) - (x < 0))
#define TIME_STEP 0.01f

#include "FootController.hpp"
#include "StepPlanner.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/MotionPlannerOutput.hpp"
#include "Data/MotionRequest.hpp"
#include "Framework/DebugDatabase.hpp"
#include "Tools/Math/Pose.hpp"

enum WalkingType
{
  STAND,
  PREPARING_STAND,
  STEPPING
};


struct ComPosition
{
  Vector3f fromLeft;
  Vector3f fromRight;
};

struct ComOffset
{
  Vector2f offsetFromRightX;
  Vector2f offsetFromLeftX;
  Vector2f offsetFromRightY;
  Vector2f offsetFromLeftY;
};

class Pendulum
{
public:
  Pendulum(const ModuleBase& module, const MotionPlannerOutput& motionPlannerOutput, const IMUSensorData& imuSensorData, DebugDatabase::DebugMap& debug);
  ~Pendulum();


  void updateParameters(const ComOffset& offset, const bool& fromStand, const bool startStepping, const InWalkKickType requestedKickType);

  void computeExpectedCom(ComPosition& comPos, float& angleL, float& angleR);
  void computeStandCom(ComPosition& comPos);

  void updateRequest(WalkingType type);
  void reset();

  void timeStep();

  Pose getTorsoMatrixChange();

  float getTimePercentage();

  bool isAborted();

  supportFoot getSupport() const;

private:
  void switchSupport(const ComOffset& offset);

  void getStep(const float& time, Vector3f& step, float& angle);

  const Parameter<Vector2f> origin_;
  const Parameter<float> periodDuration_;
  const Parameter<float> height_;
  const Parameter<float> gravity_;
  const Parameter<float> speedCorrection_;
  const Parameter<float> stepDeadTime_;
  const Parameter<Vector2f> originLimit_;
  const Parameter<float> swingLimitY_;
  const Parameter<float> stepLimitX_;
  const Parameter<float> maxAngleVelY_;
  const Parameter<bool> setDynamicSteps_;

  const IMUSensorData& imuSensorData_;
  DebugDatabase::DebugMap& debug_;

  FootController footController_;

  float zmpLimit_;
  float maxImuError_;
  float maxLastImuError_;
  float maxImuVel_;
  float maxLastImuVel_;
  float stepDampX_;
  float dynamicStepAccumulator_;
  bool isStepLimDone_;
  InWalkKickType requestedKickType_;
  InWalkKickType currentPhaseKickType_;

  struct PendulumStates
  {
    /// time
    float time; // current time
    float te;   // end time of current phase
    float tb;   // start time of current phase
    float tbb;  // start time of next phase
    float k;


    /// states for both directions
    Vector2f com{0, 0};
    Vector2f vel{0, 0};

    Pose step;
    Pose lastStep;
    Pose desiredStep;

    /// states for x-direction (spline parameters)
    float p0;  // position at start of phase
    float p1;  // position at end of phase
    float m0;  // slope at start of phase
    float m1;  // slope at end of phase
    float zmp; // the current zmp position

    /// states for y-direction (inverted pendulum)
    float x0y;  // position of com at t = 0
    float x0by; // position of com at t = 0 for next phase
    float ry;   // origin
    float rby;  // origin for next phase

    /// Kalman corrections
    Vector2f corrX{0, 0};
    Vector2f corrY{0, 0};

    /// Support foot
    supportFoot support;

    /// walkingType
    WalkingType request;
    bool abort;

    /// TorsoMatrix
    Pose torsoMatrixChange;

    bool stopNextStep;
  };

  /// The step planner calculates a pose for the next step to be performed
  StepPlanner stepPlanner_;
  PendulumStates* s_;
};
