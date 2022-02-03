#include "WebotsInterface.hpp"
#include "Framework/Configuration/Configuration.h"
#include "Framework/Log/Log.hpp"
#include "Hardware/Webots/WebotsCamera.hpp"
#include <cmath>
#include <mutex>
#include <webots/Accelerometer.hpp>
#include <webots/Camera.hpp>
#include <webots/DistanceSensor.hpp>
#include <webots/Gyro.hpp>
#include <webots/InertialUnit.hpp>
#include <webots/Keyboard.hpp>
#include <webots/LED.hpp>
#include <webots/Motor.hpp>
#include <webots/PositionSensor.hpp>
#include <webots/TouchSensor.hpp>

// NOLINTNEXTLINE(readability-function-cognitive-complexity)
WebotsInterface::WebotsInterface()
  : topCamera_{Robot::getCamera("CameraTop"), CameraPosition::TOP}
  , bottomCamera_{Robot::getCamera("CameraBottom"), CameraPosition::BOTTOM}
  , fakeData_{*this}
{
  // IMU
  accelerometer_ = getAccelerometer("IMU accelerometer");
  assert(accelerometer_ != nullptr);
  accelerometer_->enable(timeStepMilliseconds__);
  gyroscope_ = getGyro("IMU gyro");
  assert(gyroscope_ != nullptr);
  gyroscope_->enable(timeStepMilliseconds__);
  inertialUnit_ = getInertialUnit("IMU inertial");
  assert(inertialUnit_ != nullptr);
  inertialUnit_->enable(timeStepMilliseconds__);

  // sonars
  distanceSensorLeft_ = getDistanceSensor("Sonar/Left");
  assert(distanceSensorLeft_ != nullptr);
  distanceSensorLeft_->enable(timeStepMilliseconds__);
  distanceSensorRight_ = getDistanceSensor("Sonar/Right");
  assert(distanceSensorRight_ != nullptr);
  distanceSensorRight_->enable(timeStepMilliseconds__);

  // FSRs
  leftFsrFrontLeft_ = getTouchSensor("LFoot/FSR/FrontLeft");
  assert(leftFsrFrontLeft_ != nullptr);
  leftFsrFrontLeft_->enable(timeStepMilliseconds__);
  leftFsrRearLeft_ = getTouchSensor("LFoot/FSR/RearLeft");
  assert(leftFsrRearLeft_ != nullptr);
  leftFsrRearLeft_->enable(timeStepMilliseconds__);
  leftFsrFrontRight_ = getTouchSensor("LFoot/FSR/FrontRight");
  assert(leftFsrFrontRight_ != nullptr);
  leftFsrFrontRight_->enable(timeStepMilliseconds__);
  leftFsrRearRight_ = getTouchSensor("LFoot/FSR/RearRight");
  assert(leftFsrRearRight_ != nullptr);
  leftFsrRearRight_->enable(timeStepMilliseconds__);
  rightFsrFrontLeft_ = getTouchSensor("RFoot/FSR/FrontLeft");
  assert(rightFsrFrontLeft_ != nullptr);
  rightFsrFrontLeft_->enable(timeStepMilliseconds__);
  rightFsrRearLeft_ = getTouchSensor("RFoot/FSR/RearLeft");
  assert(rightFsrRearLeft_ != nullptr);
  rightFsrRearLeft_->enable(timeStepMilliseconds__);
  rightFsrFrontRight_ = getTouchSensor("RFoot/FSR/FrontRight");
  assert(rightFsrFrontRight_ != nullptr);
  rightFsrFrontRight_->enable(timeStepMilliseconds__);
  rightFsrRearRight_ = getTouchSensor("RFoot/FSR/RearRight");
  assert(rightFsrRearRight_ != nullptr);
  rightFsrRearRight_->enable(timeStepMilliseconds__);

  // motors and position sensors
  motors_[Joints::HEAD_YAW] = getMotor("HeadYaw");
  assert(motors_[Joints::HEAD_YAW] != nullptr);
  positionSensors_[Joints::HEAD_YAW] = getPositionSensor("HeadYaw_sensor");
  assert(positionSensors_[Joints::HEAD_YAW] != nullptr);
  positionSensors_[Joints::HEAD_YAW]->enable(timeStepMilliseconds__);
  motors_[Joints::HEAD_PITCH] = getMotor("HeadPitch");
  assert(motors_[Joints::HEAD_PITCH] != nullptr);
  positionSensors_[Joints::HEAD_PITCH] = getPositionSensor("HeadPitch_sensor");
  assert(positionSensors_[Joints::HEAD_PITCH] != nullptr);
  positionSensors_[Joints::HEAD_PITCH]->enable(timeStepMilliseconds__);
  motors_[Joints::L_SHOULDER_PITCH] = getMotor("LShoulderPitch");
  assert(motors_[Joints::L_SHOULDER_PITCH] != nullptr);
  positionSensors_[Joints::L_SHOULDER_PITCH] = getPositionSensor("LShoulderPitch_sensor");
  assert(positionSensors_[Joints::L_SHOULDER_PITCH] != nullptr);
  positionSensors_[Joints::L_SHOULDER_PITCH]->enable(timeStepMilliseconds__);
  motors_[Joints::L_SHOULDER_ROLL] = getMotor("LShoulderRoll");
  assert(motors_[Joints::L_SHOULDER_ROLL] != nullptr);
  positionSensors_[Joints::L_SHOULDER_ROLL] = getPositionSensor("LShoulderRoll_sensor");
  assert(positionSensors_[Joints::L_SHOULDER_ROLL] != nullptr);
  positionSensors_[Joints::L_SHOULDER_ROLL]->enable(timeStepMilliseconds__);
  motors_[Joints::L_ELBOW_YAW] = getMotor("LElbowYaw");
  assert(motors_[Joints::L_ELBOW_YAW] != nullptr);
  positionSensors_[Joints::L_ELBOW_YAW] = getPositionSensor("LElbowYaw_sensor");
  assert(positionSensors_[Joints::L_ELBOW_YAW] != nullptr);
  positionSensors_[Joints::L_ELBOW_YAW]->enable(timeStepMilliseconds__);
  motors_[Joints::L_ELBOW_ROLL] = getMotor("LElbowRoll");
  assert(motors_[Joints::L_ELBOW_ROLL] != nullptr);
  positionSensors_[Joints::L_ELBOW_ROLL] = getPositionSensor("LElbowRoll_sensor");
  assert(positionSensors_[Joints::L_ELBOW_ROLL] != nullptr);
  positionSensors_[Joints::L_ELBOW_ROLL]->enable(timeStepMilliseconds__);
  motors_[Joints::L_WRIST_YAW] = getMotor("LWristYaw");
  assert(motors_[Joints::L_WRIST_YAW] != nullptr);
  positionSensors_[Joints::L_WRIST_YAW] = getPositionSensor("LWristYaw_sensor");
  assert(positionSensors_[Joints::L_WRIST_YAW] != nullptr);
  positionSensors_[Joints::L_WRIST_YAW]->enable(timeStepMilliseconds__);
  motors_[Joints::L_HAND] = getMotor("LHand");
  assert(motors_[Joints::L_HAND] != nullptr);
  positionSensors_[Joints::L_HAND] = getPositionSensor("LHand_sensor");
  assert(positionSensors_[Joints::L_HAND] != nullptr);
  positionSensors_[Joints::L_HAND]->enable(timeStepMilliseconds__);
  motors_[Joints::L_HIP_YAW_PITCH] = getMotor("LHipYawPitch");
  assert(motors_[Joints::L_HIP_YAW_PITCH] != nullptr);
  positionSensors_[Joints::L_HIP_YAW_PITCH] = getPositionSensor("LHipYawPitch_sensor");
  assert(positionSensors_[Joints::L_HIP_YAW_PITCH] != nullptr);
  positionSensors_[Joints::L_HIP_YAW_PITCH]->enable(timeStepMilliseconds__);
  motors_[Joints::L_HIP_ROLL] = getMotor("LHipRoll");
  assert(motors_[Joints::L_HIP_ROLL] != nullptr);
  positionSensors_[Joints::L_HIP_ROLL] = getPositionSensor("LHipRoll_sensor");
  assert(positionSensors_[Joints::L_HIP_ROLL] != nullptr);
  positionSensors_[Joints::L_HIP_ROLL]->enable(timeStepMilliseconds__);
  motors_[Joints::L_HIP_PITCH] = getMotor("LHipPitch");
  assert(motors_[Joints::L_HIP_PITCH] != nullptr);
  positionSensors_[Joints::L_HIP_PITCH] = getPositionSensor("LHipPitch_sensor");
  assert(positionSensors_[Joints::L_HIP_PITCH] != nullptr);
  positionSensors_[Joints::L_HIP_PITCH]->enable(timeStepMilliseconds__);
  motors_[Joints::L_KNEE_PITCH] = getMotor("LKneePitch");
  assert(motors_[Joints::L_KNEE_PITCH] != nullptr);
  positionSensors_[Joints::L_KNEE_PITCH] = getPositionSensor("LKneePitch_sensor");
  assert(positionSensors_[Joints::L_KNEE_PITCH] != nullptr);
  positionSensors_[Joints::L_KNEE_PITCH]->enable(timeStepMilliseconds__);
  motors_[Joints::L_ANKLE_PITCH] = getMotor("LAnklePitch");
  assert(motors_[Joints::L_ANKLE_PITCH] != nullptr);
  positionSensors_[Joints::L_ANKLE_PITCH] = getPositionSensor("LAnklePitch_sensor");
  assert(positionSensors_[Joints::L_ANKLE_PITCH] != nullptr);
  positionSensors_[Joints::L_ANKLE_PITCH]->enable(timeStepMilliseconds__);
  motors_[Joints::L_ANKLE_ROLL] = getMotor("LAnkleRoll");
  assert(motors_[Joints::L_ANKLE_ROLL] != nullptr);
  positionSensors_[Joints::L_ANKLE_ROLL] = getPositionSensor("LAnkleRoll_sensor");
  assert(positionSensors_[Joints::L_ANKLE_ROLL] != nullptr);
  positionSensors_[Joints::L_ANKLE_ROLL]->enable(timeStepMilliseconds__);
  motors_[Joints::R_HIP_YAW_PITCH] = getMotor("RHipYawPitch");
  assert(motors_[Joints::R_HIP_YAW_PITCH] != nullptr);
  positionSensors_[Joints::R_HIP_YAW_PITCH] = getPositionSensor("RHipYawPitch_sensor");
  assert(positionSensors_[Joints::R_HIP_YAW_PITCH] != nullptr);
  positionSensors_[Joints::R_HIP_YAW_PITCH]->enable(timeStepMilliseconds__);
  motors_[Joints::R_HIP_ROLL] = getMotor("RHipRoll");
  assert(motors_[Joints::R_HIP_ROLL] != nullptr);
  positionSensors_[Joints::R_HIP_ROLL] = getPositionSensor("RHipRoll_sensor");
  assert(positionSensors_[Joints::R_HIP_ROLL] != nullptr);
  positionSensors_[Joints::R_HIP_ROLL]->enable(timeStepMilliseconds__);
  motors_[Joints::R_HIP_PITCH] = getMotor("RHipPitch");
  assert(motors_[Joints::R_HIP_PITCH] != nullptr);
  positionSensors_[Joints::R_HIP_PITCH] = getPositionSensor("RHipPitch_sensor");
  assert(positionSensors_[Joints::R_HIP_PITCH] != nullptr);
  positionSensors_[Joints::R_HIP_PITCH]->enable(timeStepMilliseconds__);
  motors_[Joints::R_KNEE_PITCH] = getMotor("RKneePitch");
  assert(motors_[Joints::R_KNEE_PITCH] != nullptr);
  positionSensors_[Joints::R_KNEE_PITCH] = getPositionSensor("RKneePitch_sensor");
  assert(positionSensors_[Joints::R_KNEE_PITCH] != nullptr);
  positionSensors_[Joints::R_KNEE_PITCH]->enable(timeStepMilliseconds__);
  motors_[Joints::R_ANKLE_PITCH] = getMotor("RAnklePitch");
  assert(motors_[Joints::R_ANKLE_PITCH] != nullptr);
  positionSensors_[Joints::R_ANKLE_PITCH] = getPositionSensor("RAnklePitch_sensor");
  assert(positionSensors_[Joints::R_ANKLE_PITCH] != nullptr);
  positionSensors_[Joints::R_ANKLE_PITCH]->enable(timeStepMilliseconds__);
  motors_[Joints::R_ANKLE_ROLL] = getMotor("RAnkleRoll");
  assert(motors_[Joints::R_ANKLE_ROLL] != nullptr);
  positionSensors_[Joints::R_ANKLE_ROLL] = getPositionSensor("RAnkleRoll_sensor");
  assert(positionSensors_[Joints::R_ANKLE_ROLL] != nullptr);
  positionSensors_[Joints::R_ANKLE_ROLL]->enable(timeStepMilliseconds__);
  motors_[Joints::R_SHOULDER_PITCH] = getMotor("RShoulderPitch");
  assert(motors_[Joints::R_SHOULDER_PITCH] != nullptr);
  positionSensors_[Joints::R_SHOULDER_PITCH] = getPositionSensor("RShoulderPitch_sensor");
  assert(positionSensors_[Joints::R_SHOULDER_PITCH] != nullptr);
  positionSensors_[Joints::R_SHOULDER_PITCH]->enable(timeStepMilliseconds__);
  motors_[Joints::R_SHOULDER_ROLL] = getMotor("RShoulderRoll");
  assert(motors_[Joints::R_SHOULDER_ROLL] != nullptr);
  positionSensors_[Joints::R_SHOULDER_ROLL] = getPositionSensor("RShoulderRoll_sensor");
  assert(positionSensors_[Joints::R_SHOULDER_ROLL] != nullptr);
  positionSensors_[Joints::R_SHOULDER_ROLL]->enable(timeStepMilliseconds__);
  motors_[Joints::R_ELBOW_YAW] = getMotor("RElbowYaw");
  assert(motors_[Joints::R_ELBOW_YAW] != nullptr);
  positionSensors_[Joints::R_ELBOW_YAW] = getPositionSensor("RElbowYaw_sensor");
  assert(positionSensors_[Joints::R_ELBOW_YAW] != nullptr);
  positionSensors_[Joints::R_ELBOW_YAW]->enable(timeStepMilliseconds__);
  motors_[Joints::R_ELBOW_ROLL] = getMotor("RElbowRoll");
  assert(motors_[Joints::R_ELBOW_ROLL] != nullptr);
  positionSensors_[Joints::R_ELBOW_ROLL] = getPositionSensor("RElbowRoll_sensor");
  assert(positionSensors_[Joints::R_ELBOW_ROLL] != nullptr);
  positionSensors_[Joints::R_ELBOW_ROLL]->enable(timeStepMilliseconds__);
  motors_[Joints::R_WRIST_YAW] = getMotor("RWristYaw");
  assert(motors_[Joints::R_WRIST_YAW] != nullptr);
  positionSensors_[Joints::R_WRIST_YAW] = getPositionSensor("RWristYaw_sensor");
  assert(positionSensors_[Joints::R_WRIST_YAW] != nullptr);
  positionSensors_[Joints::R_WRIST_YAW]->enable(timeStepMilliseconds__);
  motors_[Joints::R_HAND] = getMotor("RHand");
  assert(motors_[Joints::R_HAND] != nullptr);
  positionSensors_[Joints::R_HAND] = getPositionSensor("RHand_sensor");
  assert(positionSensors_[Joints::R_HAND] != nullptr);
  positionSensors_[Joints::R_HAND]->enable(timeStepMilliseconds__);

  // keyboard
  keyboard_ = getKeyboard();
  assert(keyboard_ != nullptr);
  keyboard_->enable(timeStepMilliseconds__);
}

void WebotsInterface::terminate()
{
  {
    std::lock_guard lock{terminationRequestedMutex_};
    terminationRequested_ = true;
  }
  terminationRequestedConditionVariable_.notify_all();
}

void WebotsInterface::waitForTermination()
{
  std::unique_lock lock{terminationRequestedMutex_};
  terminationRequestedConditionVariable_.wait(lock, [this] { return terminationRequested_; });
}

void WebotsInterface::configure(Configuration& configuration)
{
  robotInfo_.bodyVersion = RobotVersion::V6;
  robotInfo_.headVersion = RobotVersion::V6;
  robotInfo_.bodyName = getName();
  robotInfo_.headName = getName();
  configuration.setNaoHeadName(robotInfo_.headName);
  configuration.setNaoBodyName(robotInfo_.bodyName);
  Uni::Value value{Uni::Value(Uni::ValueType::OBJECT)};
  value << robotInfo_;
  configuration.set("tuhhSDK.base", "RobotInfo", value);

  robotMetrics_.configure(configuration, robotInfo_);
}

void WebotsInterface::setJointAngles(const JointsArray<float>& angles)
{
  for (std::size_t i{0}; i < static_cast<std::size_t>(Joints::MAX); ++i)
  {
    const auto joint{static_cast<Joints>(i)};
    requestedAngles_[joint] = angles[joint];
  }
}

void WebotsInterface::setJointStiffnesses([[maybe_unused]] const JointsArray<float>& stiffnesses)
{
  // Webots does not have stiffnesses
}

void WebotsInterface::setLEDs([[maybe_unused]] const Led::Chest& chest,
                              [[maybe_unused]] const Led::Ear& leftEar,
                              [[maybe_unused]] const Led::Ear& rightEar,
                              [[maybe_unused]] const Led::Eye& leftEye,
                              [[maybe_unused]] const Led::Eye& rightEye,
                              [[maybe_unused]] const Led::Foot& leftFoot,
                              [[maybe_unused]] const Led::Foot& rightFoot)
{
  // Webots does not have LEDs
}

void WebotsInterface::produceSensorData(CycleInfo& cycleInfo, FSRSensorData& fsrSensorData,
                                        IMUSensorData& imuSensorData,
                                        JointSensorData& jointSensorData, ButtonData& buttonData,
                                        SonarSensorData& sonarSensorData)
{
  // send motor positions
  for (std::size_t i{0}; i < static_cast<std::size_t>(Joints::MAX); ++i)
  {
    const auto joint{static_cast<Joints>(i)};
    motors_[joint]->setPosition(requestedAngles_[joint]);
  }

  {
    std::lock_guard lock{fakeData_};
    // advance simulation
    if (step(timeStepMilliseconds__) == -1)
    {
      terminate();
    }
  }

  // set simulation time
  cycleInfo.startTime = Clock::time_point{Clock::duration{static_cast<float>(getTime())}};
  cycleInfo.cycleTime = cycleInfo.startTime - lastSensorDataProduction_;
  cycleInfo.valid = true;
  lastSensorDataProduction_ = cycleInfo.startTime;

  // read motor positions and states
  for (std::size_t i{0}; i < static_cast<std::size_t>(Joints::MAX); ++i)
  {
    const auto joint{static_cast<Joints>(i)};
    jointSensorData.angles[joint] = static_cast<float>(positionSensors_[joint]->getValue());
    jointSensorData.currents[joint] = 0.f;
    jointSensorData.temperatures[joint] = 30.f;
  }
  jointSensorData.valid = true;

  // read IMU
  const double* accelerometerValues{accelerometer_->getValues()};
  imuSensorData.accelerometer.x() = static_cast<float>(accelerometerValues[0]);
  imuSensorData.accelerometer.y() = static_cast<float>(accelerometerValues[1]);
  imuSensorData.accelerometer.z() = static_cast<float>(accelerometerValues[2]);
  const double* gyroscopeValues{gyroscope_->getValues()};
  imuSensorData.gyroscope.x() = static_cast<float>(gyroscopeValues[0]);
  imuSensorData.gyroscope.y() = static_cast<float>(gyroscopeValues[1]);
  imuSensorData.gyroscope.z() = static_cast<float>(gyroscopeValues[2]);
  const double* angleValues{inertialUnit_->getRollPitchYaw()};
  imuSensorData.angle.x() = static_cast<float>(angleValues[0]);
  imuSensorData.angle.y() = static_cast<float>(angleValues[1]);
  // imuSensorData.angle.z() = static_cast<float>(angleValues[2]);

  // read sonars
  sonarSensorData.data.leftSensor = static_cast<float>(distanceSensorLeft_->getValue());
  sonarSensorData.data.rightSensor = static_cast<float>(distanceSensorRight_->getValue());
  sonarSensorData.valid = {{true, true}};

  // read FSRs
  fsrSensorData.leftFoot.frontLeft = static_cast<float>(leftFsrFrontLeft_->getValues()[2]);
  fsrSensorData.leftFoot.frontRight = static_cast<float>(leftFsrRearLeft_->getValues()[2]);
  fsrSensorData.leftFoot.rearLeft = static_cast<float>(leftFsrFrontRight_->getValues()[2]);
  fsrSensorData.leftFoot.rearRight = static_cast<float>(leftFsrRearRight_->getValues()[2]);
  fsrSensorData.rightFoot.frontLeft = static_cast<float>(rightFsrFrontLeft_->getValues()[2]);
  fsrSensorData.rightFoot.frontRight = static_cast<float>(rightFsrRearLeft_->getValues()[2]);
  fsrSensorData.rightFoot.rearLeft = static_cast<float>(rightFsrFrontRight_->getValues()[2]);
  fsrSensorData.rightFoot.rearRight = static_cast<float>(rightFsrRearRight_->getValues()[2]);
  fsrSensorData.totalLeft = fsrSensorData.leftFoot.frontLeft + fsrSensorData.leftFoot.frontRight +
                            fsrSensorData.leftFoot.rearLeft + fsrSensorData.leftFoot.rearRight;
  fsrSensorData.rightFoot = fsrSensorData.rightFoot;
  fsrSensorData.totalRight = fsrSensorData.rightFoot.frontLeft +
                             fsrSensorData.rightFoot.frontRight + fsrSensorData.rightFoot.rearLeft +
                             fsrSensorData.rightFoot.rearRight;
  fsrSensorData.valid = true;

  // read keyboard (for switches)
  const auto key{keyboard_->getKey()};
  if (key != -1)
  {
    // NOLINTNEXTLINE(hicpp-signed-bitwise)
    if (key == (webots::Keyboard::CONTROL | webots::Keyboard::SHIFT | 'C'))
    {
      buttonData.switches.isChestButtonPressed = true;
    }
  }
  bool singlePressDetected{!buttonData.switches.isChestButtonPressed &&
                           chestButtonWasPressedLastCycle_};
  if (singlePressDetected)
  {
    lastChestButtonSinglePress_ = cycleInfo.startTime;
  }
  buttonData.lastChestButtonSinglePress = lastChestButtonSinglePress_;
  buttonData.valid = true;
  chestButtonWasPressedLastCycle_ = buttonData.switches.isChestButtonPressed;

  // retrieve requested images
  bool expected{true};
  if (topCameraRequested_.compare_exchange_weak(expected, false))
  {
    topCamera_.updateImage(cycleInfo.startTime);
  }
  expected = true;
  if (bottomCameraRequested_.compare_exchange_weak(expected, false))
  {
    bottomCamera_.updateImage(cycleInfo.startTime);
  }
}

void WebotsInterface::enableImageDataProducer()
{
  topCamera_.enable();
  bottomCamera_.enable();
}

void WebotsInterface::disableImageDataProducer()
{
  topCamera_.disable();
  bottomCamera_.disable();
}

void WebotsInterface::produceImageData(CycleInfo& cycleInfo, ImageData& imageData)
{
  // check the last position to now produce the other one
  if (lastRequestedCameraPosition_ == CameraPosition::TOP)
  {
    bottomCameraRequested_.store(true);
    lastRequestedCameraPosition_ = CameraPosition::BOTTOM;
    bottomCamera_.produce(cycleInfo, imageData);
  }
  else
  {
    topCameraRequested_.store(true);
    lastRequestedCameraPosition_ = CameraPosition::TOP;
    topCamera_.produce(cycleInfo, imageData);
  }

  cycleInfo.cycleTime = cycleInfo.startTime - lastImageDataProduction_;
  cycleInfo.valid = true;
  lastImageDataProduction_ = cycleInfo.startTime;
}

std::string WebotsInterface::getFileRoot() const
{
  return LOCAL_FILE_ROOT;
}

std::string WebotsInterface::getDataRoot() const
{
  return LOCAL_FILE_ROOT;
}

const RobotInfo& WebotsInterface::getRobotInfo()
{
  return robotInfo_;
}

const RobotMetrics& WebotsInterface::getRobotMetrics()
{
  return robotMetrics_;
}

FakeDataInterface& WebotsInterface::getFakeData()
{
  return fakeData_;
}

AudioInterface& WebotsInterface::getAudio()
{
  return audio_;
}
