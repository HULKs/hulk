#pragma once

#include <array>
#include <atomic>
#include <cinttypes>

namespace ProxyInterface
{

  struct RobotConfiguration
  {
    std::array<std::uint8_t, 20> bodyId{};
    std::uint8_t bodyVersion{};
    std::array<std::uint8_t, 20> headId{};
    std::uint8_t headVersion{};
  };

  struct Vertex2
  {
    float x{};
    float y{};
  };

  struct Vertex3
  {
    float x{};
    float y{};
    float z{};
  };

  struct InertialMeasurementUnit
  {
    Vertex3 accelerometer;
    Vertex2 angles;
    Vertex3 gyroscope;
  };

  struct ForceSensitiveResistors
  {
    float leftFootFrontLeft{};
    float leftFootFrontRight{};
    float leftFootRearLeft{};
    float leftFootRearRight{};
    float rightFootFrontLeft{};
    float rightFootFrontRight{};
    float rightFootRearLeft{};
    float rightFootRearRight{};
  };

  struct TouchSensors
  {
    bool chestButton{};
    bool headFront{};
    bool headMiddle{};
    bool headRear{};
    bool leftFootLeft{};
    bool leftFootRight{};
    bool leftHandBack{};
    bool leftHandLeft{};
    bool leftHandRight{};
    bool rightFootLeft{};
    bool rightFootRight{};
    bool rightHandBack{};
    bool rightHandLeft{};
    bool rightHandRight{};
  };

  struct SonarSensors
  {
    float left{};
    float right{};
  };

  struct JointsArray
  {
    float headYaw{};
    float headPitch{};
    float leftShoulderPitch{};
    float leftShoulderRoll{};
    float leftElbowYaw{};
    float leftElbowRoll{};
    float leftWristYaw{};
    float leftHipYawPitch{};
    float leftHipRoll{};
    float leftHipPitch{};
    float leftKneePitch{};
    float leftAnklePitch{};
    float leftAnkleRoll{};
    float rightHipRoll{};
    float rightHipPitch{};
    float rightKneePitch{};
    float rightAnklePitch{};
    float rightAnkleRoll{};
    float rightShoulderPitch{};
    float rightShoulderRoll{};
    float rightElbowYaw{};
    float rightElbowRoll{};
    float rightWristYaw{};
    float leftHand{};
    float rightHand{};
  };

  struct StateStorage
  {
    /// Seconds since proxy start
    float receivedAt{};
    RobotConfiguration robotConfiguration;
    InertialMeasurementUnit inertialMeasurementUnit;
    ForceSensitiveResistors forceSensitiveResistors;
    TouchSensors touchSensors;
    SonarSensors sonarSensors;
    JointsArray position;
    JointsArray stiffness;
    JointsArray current;
    JointsArray temperature;
    JointsArray status;
  };

  struct Color
  {
    float red{};
    float green{};
    float blue{};
  };

  struct Eye
  {
    Color colorAt0;
    Color colorAt45;
    Color colorAt90;
    Color colorAt135;
    Color colorAt180;
    Color colorAt225;
    Color colorAt270;
    Color colorAt315;
  };

  struct Ear
  {
    float intensityAt0{};
    float intensityAt36{};
    float intensityAt72{};
    float intensityAt108{};
    float intensityAt144{};
    float intensityAt180{};
    float intensityAt216{};
    float intensityAt252{};
    float intensityAt288{};
    float intensityAt324{};
  };

  struct ControlStorage
  {
    Eye leftEye;
    Eye rightEye;
    Color chest;
    Color leftFoot;
    Color rightFoot;
    Ear leftEar;
    Ear rightEar;
    JointsArray position;
    JointsArray stiffness;
  };

} // namespace ProxyInterface
