#include "Nao6Interface.hpp"

#include <boost/filesystem.hpp>
#include <iostream>
#include <mntent.h>
#include <numeric>
#include <thread>

#include "Modules/Configuration/Configuration.h"
#include "print.h"

NaoInterface::NaoInterface()
  : lolaDesync_(false)
  , fragmentSize_(0)
  , ioService_()
  , socket_(ioService_)
  , lolaEndpoint_("/tmp/robocup")
  , backgroundThread_()
  , newNetworkData_(false)
  , previousChestButtonState_(0)
  , previousFrontHeadState_(0)
  , previousRearHeadState_(0)
  , sentChestButton_(true)
  , topCamera_(Camera::TOP)
  , bottomCamera_(Camera::BOTTOM)
  , currentCamera_(Camera::TOP)
  , currentUsedImageTimeStamp(0)
  , lastUsedImageTimeStamp(0)
{
  receive_ = std::make_shared<LoLADataBuffer>();
  send_ = std::make_shared<LoLADataBuffer>();

  receive_->fill(0);
  send_->fill(0);

  while (!boost::filesystem::exists(lolaEndpoint_.path()))
  {
    Log(LogLevel::INFO) << "Waiting for lola socket to be available";
    std::this_thread::sleep_for(std::chrono::milliseconds(500));
  }
  socket_.connect(lolaEndpoint_);


  // Manage the remapping of the joint data
  {
    using namespace keys::joints;
    using namespace keys::sensor;
    using namespace keys::led;
    // Don't forget R_HIP_YAW_PITCH
    jointsRemapping_ = {
        HEAD_YAW,     HEAD_PITCH,    L_SHOULDER_PITCH, L_SHOULDER_ROLL,  L_ELBOW_YAW,
        L_ELBOW_ROLL, L_WRIST_YAW,   L_HIP_YAW_PITCH,  L_HIP_ROLL,       L_HIP_PITCH,
        L_KNEE_PITCH, L_ANKLE_PITCH, L_ANKLE_ROLL,     R_HIP_ROLL,       R_HIP_PITCH,
        R_KNEE_PITCH, R_ANKLE_PITCH, R_ANKLE_ROLL,     R_SHOULDER_PITCH, R_SHOULDER_ROLL,
        R_ELBOW_YAW,  R_ELBOW_ROLL,  R_WRIST_YAW,      L_HAND,           R_HAND};
    batteryRemapping_ = {BATTERY_CHARGE, BATTERY_STATUS, BATTERY_CURRENT, BATTERY_TEMPERATURE};
    switchesRemapping_ = {SWITCH_CHEST_BUTTON, SWITCH_HEAD_FRONT,   SWITCH_HEAD_MIDDLE,
                          SWITCH_HEAD_REAR,    SWITCH_L_FOOT_LEFT,  SWITCH_L_FOOT_RIGHT,
                          SWITCH_L_HAND_BACK,  SWITCH_L_HAND_LEFT,  SWITCH_L_HAND_RIGHT,
                          SWITCH_R_FOOT_LEFT,  SWITCH_R_FOOT_RIGHT, SWITCH_R_HAND_BACK,
                          SWITCH_R_HAND_LEFT,  SWITCH_R_HAND_RIGHT};

    colorRemapping_ = {2, 1, 0};
    lEarRemapping_ = {EAR_DEG_0,   EAR_DEG_36,  EAR_DEG_72,  EAR_DEG_108, EAR_DEG_144,
                      EAR_DEG_180, EAR_DEG_216, EAR_DEG_252, EAR_DEG_288, EAR_DEG_324};
    rEarRemapping_ = {EAR_DEG_324, EAR_DEG_288, EAR_DEG_252, EAR_DEG_216, EAR_DEG_180,
                      EAR_DEG_144, EAR_DEG_108, EAR_DEG_72,  EAR_DEG_36,  EAR_DEG_0};
    skullRemapping_ = {HEAD_REAR_RIGHT_2,   HEAD_REAR_RIGHT_1,  HEAD_REAR_RIGHT_0,
                       HEAD_REAR_LEFT_2,    HEAD_REAR_LEFT_1,   HEAD_REAR_LEFT_0,
                       HEAD_MIDDLE_RIGHT_0, HEAD_MIDDLE_LEFT_0, HEAD_FRONT_RIGHT_1,
                       HEAD_FRONT_RIGHT_0,  HEAD_FRONT_LEFT_1,  HEAD_FRONT_LEFT_0};
    lEyeRemapping_ = {EYE_RED_DEG_45,    EYE_RED_DEG_0,     EYE_RED_DEG_315,   EYE_RED_DEG_270,
                      EYE_RED_DEG_225,   EYE_RED_DEG_180,   EYE_RED_DEG_135,   EYE_RED_DEG_90,
                      EYE_GREEN_DEG_45,  EYE_GREEN_DEG_0,   EYE_GREEN_DEG_315, EYE_GREEN_DEG_270,
                      EYE_GREEN_DEG_225, EYE_GREEN_DEG_180, EYE_GREEN_DEG_135, EYE_GREEN_DEG_90,
                      EYE_BLUE_DEG_45,   EYE_BLUE_DEG_0,    EYE_BLUE_DEG_315,  EYE_BLUE_DEG_270,
                      EYE_BLUE_DEG_225,  EYE_BLUE_DEG_180,  EYE_BLUE_DEG_135,  EYE_BLUE_DEG_90};
    rEyeRemapping_ = {EYE_RED_DEG_0,     EYE_RED_DEG_45,    EYE_RED_DEG_90,    EYE_RED_DEG_135,
                      EYE_RED_DEG_180,   EYE_RED_DEG_225,   EYE_RED_DEG_270,   EYE_RED_DEG_315,
                      EYE_GREEN_DEG_0,   EYE_GREEN_DEG_45,  EYE_GREEN_DEG_90,  EYE_GREEN_DEG_135,
                      EYE_GREEN_DEG_180, EYE_GREEN_DEG_225, EYE_GREEN_DEG_270, EYE_GREEN_DEG_315,
                      EYE_BLUE_DEG_0,    EYE_BLUE_DEG_45,   EYE_BLUE_DEG_90,   EYE_BLUE_DEG_135,
                      EYE_BLUE_DEG_180,  EYE_BLUE_DEG_225,  EYE_BLUE_DEG_270,  EYE_BLUE_DEG_315};
  }

  auto extractNaoInfoMapValue = [&](msgpack::object& value, keys::naoinfos::naoinfo dst) {
    if (value.type != msgpack::type::STR)
    {
      throw std::runtime_error("Expected string type");
    }
    value >> dataBlock_.naoInfoKey[dst];
  };

  // Receive the first answer from lola
  std::size_t numberOfBytes = socket_.receive(boost::asio::buffer(*receive_));
  msgpack::object_handle ob = msgpack::unpack(receive_->data(), numberOfBytes);
  msgpack::object obj = ob.get();

  msgpack::object& robotConfig = obj.via.map.ptr[0].val;

  try
  {
    extractNaoInfoMapValue(robotConfig.via.array.ptr[0], keys::naoinfos::BODY_ID);
    extractNaoInfoMapValue(robotConfig.via.array.ptr[1], keys::naoinfos::BODY_BASE_VERSION);
    extractNaoInfoMapValue(robotConfig.via.array.ptr[2], keys::naoinfos::HEAD_ID);
    extractNaoInfoMapValue(robotConfig.via.array.ptr[3], keys::naoinfos::HEAD_BASE_VERSION);
  }
  catch (std::runtime_error& err)
  {
    // Only die after dumping the msg pack object.
    Log(LogLevel::ERROR) << "Unable to extract nao info from msg pack object: " << err.what();
    Log(LogLevel::ERROR) << obj;
    throw std::runtime_error(
        "Unable to initialize nao6 interface. See log for further details, consider reboot");
  }

  registerForSocketReceive();

  // Start background thread
  backgroundThread_ = std::make_shared<std::thread>([this]() { ioService_.run(); });
}

NaoInterface::~NaoInterface()
{
  ioService_.stop();
  backgroundThread_->join();
  socket_.close();
}

void NaoInterface::configure(Configuration& config, NaoInfo& naoInfo)
{
  // This needs to be done here because now the identity of the NAO is known.
  topCamera_.configure(config, naoInfo);
  bottomCamera_.configure(config, naoInfo);
}


void NaoInterface::registerForSocketReceive()
{
  socket_.async_receive(
      boost::asio::buffer(*receive_),
      [this](const boost::system::error_code& error, const std::size_t bytesTransfered) {
        // Only accept data if there was no error and no buffer overrun.
        if (!error && bytesTransfered != receive_->size())
        {
          timeNetworkDataReceived_ = TimePoint::getCurrentTime();

          const std::size_t fragmentOffset = fragmentSize_ == 0 ? 0 : LoLADatumSize - fragmentSize_;

          // Merge last received fragment with the new data
          if (fragmentSize_ > 0)
          {
            LoLASingleDatumBuffer dataBuffer;
            // copy fragment from last cycle into dataBuffer
            std::copy(fragment_.begin(), fragment_.begin() + fragmentSize_, dataBuffer.begin());
            // copy remaining part of the fragment from the new data.
            // This assumes that we never receive a LoLA datum in 3 pieces!
            std::copy(receive_->begin(), receive_->begin() + fragmentOffset,
                      dataBuffer.begin() + fragmentSize_);
            lolaData_.push(dataBuffer);
          }

          // Copy all received LoLAData packages into the ring buffer.
          for (size_t i = 0; i < (bytesTransfered - fragmentOffset) / LoLADatumSize; ++i)
          {
            LoLASingleDatumBuffer dataBuffer;
            std::copy(receive_->begin() + fragmentOffset + i * sizeof(dataBuffer),
                      receive_->begin() + fragmentOffset + (i + 1) * sizeof(dataBuffer),
                      dataBuffer.begin());

            lolaData_.push(dataBuffer);
          }

          // Check if there is a datum fragment datum at the end of the receive buffer.
          if (bytesTransfered % LoLADatumSize != 0)
          {
            fragmentSize_ = bytesTransfered % LoLADatumSize;
            std::copy(receive_->begin() + (bytesTransfered - fragmentSize_), receive_->end(),
                      fragment_.begin());
          }
          else
          {
            fragmentSize_ = 0;
          }

          newNetworkDataCondition_.notify_one();
        }

        if (bytesTransfered != receive_->size())
        {
          // continue regardless of "error"
          registerForSocketReceive();
        }
        else
        {
          // Overrun on receive_. This means that we dropped an unknown amount of bytes from the
          // lola stream. We are not able to recover from this in this thread. Notify the motion
          // thread and terminate by not calling registerForSocketReceive() again.
          Log(LogLevel::ERROR) << "Lola stream desync!";
          lolaDesync_ = true;
          newNetworkDataCondition_.notify_one();
        }
      });
}

void NaoInterface::setJointAngles(const std::vector<float>& angles)
{
  assert(angles.size() == dataBlock_.commandAngles.size());
  for (unsigned int i = 0; i < angles.size(); i++)
  {
    dataBlock_.commandAngles[i] = angles[i];
  }
  dataBlock_.newAngles = true;
}

void NaoInterface::setJointStiffnesses(const std::vector<float>& stiffnesses)
{
  assert(stiffnesses.size() == dataBlock_.commandStiffnesses.size());
  for (unsigned int i = 0; i < stiffnesses.size(); i++)
  {
    dataBlock_.commandStiffnesses[i] = stiffnesses[i];
  }
  dataBlock_.newStiffnesses = true;
}

void NaoInterface::setLEDs(const std::vector<float>& leds)
{
  assert(leds.size() == dataBlock_.commandLEDs.size());
  for (unsigned int i = 0; i < leds.size(); i++)
  {
    dataBlock_.commandLEDs[i] = leds[i];
  }
}

void NaoInterface::setSonar(const float /*sonar*/)
{
  // The LoLA API does not allow to change any sonar parameters anymore.
  // The current sonar measurements are send every cycle.
}

float NaoInterface::waitAndReadSensorData(NaoSensorData& data)
{
  {
    using namespace keys::led;
    using namespace keys::sensor;
    // Update battery
    batteryDisplay_.displayBatteryCharge(
        dataBlock_.battery[BATTERY_CHARGE], dataBlock_.battery[BATTERY_CURRENT],
        dataBlock_.commandLEDs.data() + CHEST_MAX + 2 * EAR_MAX + 2 * EYE_MAX);
  }

  // assemble all data to send to LoLA
  {
    using namespace keys::led;
    sbuf_.clear();
    msgpack::packer<msgpack::sbuffer> packer(sbuf_);
    packer.pack_map(11); // 11
    packFloatArray(packer, dataBlock_.commandLEDs.data(), colorRemapping_, "Chest");
    packFloatArray(packer, dataBlock_.commandLEDs.data() + CHEST_MAX, lEarRemapping_, "LEar");
    packFloatArray(packer, dataBlock_.commandLEDs.data() + CHEST_MAX + 2 * EAR_MAX, lEyeRemapping_,
                   "LEye");
    packFloatArray(packer,
                   dataBlock_.commandLEDs.data() + CHEST_MAX + 2 * EAR_MAX + 2 * EYE_MAX + HEAD_MAX,
                   colorRemapping_, "LFoot");
    packJoints(packer, dataBlock_.commandAngles, "Position");
    packFloatArray(packer, dataBlock_.commandLEDs.data() + CHEST_MAX + EAR_MAX, rEarRemapping_,
                   "REar");
    packFloatArray(packer, dataBlock_.commandLEDs.data() + CHEST_MAX + 2 * EAR_MAX + EYE_MAX,
                   rEyeRemapping_, "REye");
    packFloatArray(packer,
                   dataBlock_.commandLEDs.data() + CHEST_MAX + 2 * EAR_MAX + 2 * EYE_MAX +
                       HEAD_MAX + FOOT_MAX,
                   colorRemapping_, "RFoot");
    packFloatArray(packer, dataBlock_.commandLEDs.data() + CHEST_MAX + 2 * EAR_MAX + 2 * EYE_MAX,
                   skullRemapping_, "Skull");

    packer.pack("Sonar");
    packer.pack_array(2);
    packer.pack_true();
    packer.pack_true();

    packJoints(packer, dataBlock_.commandStiffnesses, "Stiffness");
  }

  // actually send the assembled data to LoLA
  socket_.send(boost::asio::buffer(sbuf_.data(), sbuf_.size()));

  // Wait for an answer from LoLA (via background thread)
  {
    std::unique_lock<std::mutex> lg(mutex_);
    newNetworkDataCondition_.wait(
        lg, [this] { return lolaData_.pop(lastLolaReceivedDatum_) || lolaDesync_; });
  }

  // Check if background thread is still in sync.
  // If not so: Wait for the background thread to terminate, close the socket to LoLA and reopen it
  // after a short period of time. Then the background thread is started again.
  if (lolaDesync_)
  {
    backgroundThread_->join();
    lolaDesync_ = false;
    fragmentSize_ = 0;
    ioService_.restart();
    socket_.close();
    std::this_thread::sleep_for(std::chrono::milliseconds(50));
    socket_.connect(lolaEndpoint_);
    registerForSocketReceive();
    // Start background thread
    backgroundThread_ = std::make_shared<std::thread>([this]() { ioService_.run(); });
    std::unique_lock<std::mutex> lg(mutex_);
    newNetworkDataCondition_.wait(lg, [this] { return lolaData_.pop(lastLolaReceivedDatum_); });

    Log(LogLevel::WARNING) << "LoLA resynced!";
  }

  // parse the incoming LoLA message
  {
    msgpack::object_handle ob = msgpack::unpack(lastLolaReceivedDatum_.data(), LoLADatumSize);
    msgpack::object obj = ob.get();

    if (obj.type != msgpack::type::MAP)
    {
      Log(LogLevel::ERROR) << "Unable to parse LoLA msg: Unexpected type. Object dump:";
      Log(LogLevel::ERROR) << obj;
      throw std::runtime_error("Wrong msgpack type from LoLA, expected MAP!");
    }

    // extract RobotConfig
    extractVector3(obj.via.map.ptr[1].val, dataBlock_.imu.data() + keys::sensor::IMU_ACC_X);
    extractVector2(obj.via.map.ptr[2].val, dataBlock_.imu.data() + keys::sensor::IMU_ANGLE_X);
    extractBattery(obj.via.map.ptr[3].val, dataBlock_.battery);
    extractJoints(obj.via.map.ptr[4].val, dataBlock_.jointCurrent);
    extractFSRs(obj.via.map.ptr[5].val, dataBlock_.fsrLeft, dataBlock_.fsrRight);
    extractVector3(obj.via.map.ptr[6].val, dataBlock_.imu.data() + keys::sensor::IMU_GYR_X);
    extractJoints(obj.via.map.ptr[7].val, dataBlock_.jointSensor);
    extractSonar(obj.via.map.ptr[8].val, dataBlock_.sonar);
    // extract Stiffness(9) here if needed
    extractJoints(obj.via.map.ptr[10].val, dataBlock_.jointTemperature);
    extractSwitches(obj.via.map.ptr[11].val, dataBlock_.switches);
    extractJoints(obj.via.map.ptr[12].val, dataBlock_.jointStatus);


    const float currentFrontHeadState = dataBlock_.switches[keys::sensor::SWITCH_HEAD_FRONT];
    const float currentRearHeadState = dataBlock_.switches[keys::sensor::SWITCH_HEAD_REAR];
    const float currentChestButtonState = dataBlock_.switches[keys::sensor::SWITCH_CHEST_BUTTON];
    std::chrono::time_point<std::chrono::system_clock> now = std::chrono::system_clock::now();
    unsigned int msSinceLastFrontHeadTime =
        std::chrono::duration_cast<std::chrono::milliseconds>(now - previousFrontHeadTime_).count();
    unsigned int msSinceLastRearHeadTime =
        std::chrono::duration_cast<std::chrono::milliseconds>(now - previousRearHeadTime_).count();

    // Touched front
    if (currentFrontHeadState > previousFrontHeadState_)
    {
      if (msSinceLastRearHeadTime < 500 && msSinceLastFrontHeadTime < 1000)
      {
        dataBlock_.chestButtonPressed = false;
        dataBlock_.chestButtonDoublePressed = true;
        sentChestButton_ = true;
      }
      previousFrontHeadTime_ = now;
    }
    // Touched rear
    else if (currentRearHeadState > previousRearHeadState_ ||
             currentChestButtonState > previousChestButtonState_)
    {
      if (currentChestButtonState > previousChestButtonState_)
      {
        dataBlock_.chestButtonDoublePressed = false;
        dataBlock_.chestButtonPressed = true;
        sentChestButton_ = false;
      }
      previousRearHeadTime_ = now;
    }
    else
    {
      dataBlock_.chestButtonPressed = false;
      dataBlock_.chestButtonDoublePressed = false;
    }
    previousChestButtonState_ = currentChestButtonState;
    previousFrontHeadState_ = currentFrontHeadState;
    previousRearHeadState_ = currentRearHeadState;
  }

  // copy the received LoLA data into the given data block (data is parameter of this function)
  data.jointCurrent = dataBlock_.jointCurrent;
  data.jointSensor = dataBlock_.jointSensor;
  data.jointStatus = dataBlock_.jointStatus;
  data.jointTemperature = dataBlock_.jointTemperature;
  data.switches = dataBlock_.switches;
  data.imu = dataBlock_.imu;
  data.fsrLeft = dataBlock_.fsrLeft;
  data.fsrRight = dataBlock_.fsrRight;
  data.sonar = dataBlock_.sonar;
  data.battery = dataBlock_.battery;
  data.time = timeNetworkDataReceived_;

  data.fsrLeft[keys::sensor::FSR_TOTAL_WEIGHT] =
      std::accumulate(data.fsrLeft.begin(), data.fsrLeft.begin() + 4, 0.f);
  data.fsrRight[keys::sensor::FSR_TOTAL_WEIGHT] =
      std::accumulate(data.fsrRight.begin(), data.fsrRight.begin() + 4, 0.f);

  data.jointCurrent[keys::joints::R_HIP_YAW_PITCH] =
      dataBlock_.jointCurrent[keys::joints::L_HIP_YAW_PITCH];
  data.jointSensor[keys::joints::R_HIP_YAW_PITCH] =
      dataBlock_.jointSensor[keys::joints::L_HIP_YAW_PITCH];
  data.jointStatus[keys::joints::R_HIP_YAW_PITCH] =
      dataBlock_.jointStatus[keys::joints::L_HIP_YAW_PITCH];
  data.jointTemperature[keys::joints::R_HIP_YAW_PITCH] =
      dataBlock_.jointTemperature[keys::joints::L_HIP_YAW_PITCH];

  // Callback
  {
    if (dataBlock_.chestButtonPressed)
    {
      data.buttonCallbackList.push_back(CE_CHESTBUTTON_SIMPLE);
    }
    if (dataBlock_.chestButtonDoublePressed)
    {
      data.buttonCallbackList.push_back(CE_CHESTBUTTON_DOUBLE);
    }
  }

  newNetworkData_ = false;

  // Approximated time since last sensor reading
  return 0.012f;
}

std::string NaoInterface::getFileRoot()
{
  return "/home/nao/naoqi/";
}

std::string NaoInterface::getDataRoot()
{
  std::string fileTransportRoot = getFileRoot();
  mntent* ent;
  mntent dummy;
  char* buf = new char[4096];
  FILE* aFile = setmntent("/proc/mounts", "r");

  if (aFile != nullptr)
  {
    while (nullptr != (ent = getmntent_r(aFile, &dummy, buf, 4096)))
    {
      std::string fsname(ent->mnt_fsname);
      if (fsname == "/dev/sdb1")
      {
        fileTransportRoot = std::string(ent->mnt_dir) + "/";
        tuhhprint::print("Will use " + fileTransportRoot + " as FileTransport directory!",
                         LogLevel::FANCY);
        break;
      }
    }
    endmntent(aFile);
  }
  else
  {
    tuhhprint::print("Could not get mountpoints for FileTransport directory!", LogLevel::ERROR);
  }
  return fileTransportRoot;
}

void NaoInterface::getNaoInfo(Configuration& config, NaoInfo& info)
{
  // This method is normally called only once, just to make sure...
  if (naoInfo_.bodyName.empty())
  {
    initNaoInfo(config);
  }
  info = naoInfo_;
}

CameraInterface& NaoInterface::getCamera(const Camera camera)
{
  return (camera == Camera::TOP) ? topCamera_ : bottomCamera_;
}

CameraInterface& NaoInterface::getNextCamera()
{
  // Release last used image
  if (currentCamera_ == Camera::TOP)
  {
    topCamera_.releaseImage();
  }
  else if (currentCamera_ == Camera::BOTTOM)
  {
    bottomCamera_.releaseImage();
  }

  const bool imageAvailable = topCamera_.isImageValid() || bottomCamera_.isImageValid();
  // get new images ONLY if there is no valid one anymore.
  if (!imageAvailable)
  {
    bool allImagesAvailable = topCamera_.isImageValid() && bottomCamera_.isImageValid();
    // Wait for all images to become valid
    do
    {
      std::array<NaoCamera*, 2> cameras{{&topCamera_, &bottomCamera_}};
      if (!NaoCamera::waitForCameras(cameras, 200))
      {
        throw std::runtime_error("Something went wrong while trying to capture an image");
      }

      // If the images are older than the last used ones, get new images
      if (topCamera_.isImageValid() && topCamera_.getTimeStamp() < currentUsedImageTimeStamp)
      {
        Log(LogLevel::WARNING) << "Discarding image for TOP";
        topCamera_.releaseImage();
      }
      if (bottomCamera_.isImageValid() && bottomCamera_.getTimeStamp() < currentUsedImageTimeStamp)
      {
        Log(LogLevel::WARNING) << "Discarding image for BOTTOM";
        bottomCamera_.releaseImage();
      }
      allImagesAvailable = topCamera_.isImageValid() && bottomCamera_.isImageValid();
    } while (!allImagesAvailable);
  }


  // Use the older of the two images first
  if (topCamera_.isImageValid() &&
      (!bottomCamera_.isImageValid() || topCamera_.getTimeStamp() < bottomCamera_.getTimeStamp()))
  {
    currentCamera_ = Camera::TOP;
    currentUsedImageTimeStamp = topCamera_.getTimeStamp();
  }
  else
  {
    currentCamera_ = Camera::BOTTOM;
    currentUsedImageTimeStamp = bottomCamera_.getTimeStamp();
  }

  // Check for the correct timeline
  assert(currentUsedImageTimeStamp >= lastUsedImageTimeStamp);
  lastUsedImageTimeStamp = currentUsedImageTimeStamp;

  return currentCamera_ == Camera::TOP ? topCamera_ : bottomCamera_;
}

Camera NaoInterface::getCurrentCameraType()
{
  return currentCamera_;
}

AudioInterface& NaoInterface::getAudio()
{
  return audioInterface_;
}

FakeDataInterface& NaoInterface::getFakeData()
{
  return fakeData_;
}

void NaoInterface::initNaoInfo(Configuration& config)
{
  print("NaoInterface::initNaoInfo", LogLevel::INFO);
  std::string bodyname = "";
  std::string headname = "";

  /// ID Mapping

  std::string bodyIDstr = dataBlock_.naoInfoKey[keys::naoinfos::BODY_ID];
  std::string headIDstr = dataBlock_.naoInfoKey[keys::naoinfos::HEAD_ID];

  print("Body ID: " + bodyIDstr, LogLevel::INFO);
  print("Head ID: " + headIDstr, LogLevel::INFO);

  config.mount("NaoInterface.id_map", "id_map.json", ConfigurationType::HEAD);
  Uni::Value& val = config.get("NaoInterface.id_map", "idmap.nao");

  try
  {
    for (auto it = val.vectorBegin(); it != val.vectorEnd(); it++)
    {
      std::string bid = (*it)["bodyid"].asString();
      std::string hid = (*it)["headid"].asString();

      if (bid == bodyIDstr)
      {
        bodyname = (*it)["name"].asString();
      }

      if (hid == headIDstr)
      {
        headname = (*it)["name"].asString();
      }
    }
  }
  catch (std::exception& err)
  {
    print(std::string("Error when finding out my identity. (NaoInterface::initNaoInfo). ") +
              err.what(),
          LogLevel::ERROR);
  }

  if (bodyname.empty())
  {
    print("body could not be identified", LogLevel::ERROR);
  }
  if (headname.empty())
  {
    print("head could not be identified", LogLevel::ERROR);
  }
  if (bodyname.empty() || headname.empty())
  {
    throw std::runtime_error("Could not determine either body or headname");
  }

  naoInfo_.bodyName = bodyname;
  naoInfo_.headName = headname;

  // Determine version
  std::string bodyVersionString = dataBlock_.naoInfoKey[keys::naoinfos::BODY_BASE_VERSION];
  std::string headVersionString = dataBlock_.naoInfoKey[keys::naoinfos::HEAD_BASE_VERSION];
  if (bodyVersionString == "6.0.0")
  {
    naoInfo_.bodyVersion = NaoVersion::V6;
  }
  else if (bodyVersionString == "V5.0")
  {
    naoInfo_.bodyVersion = NaoVersion::V5;
  }
  else if (bodyVersionString == "V4.0")
  {
    naoInfo_.bodyVersion = NaoVersion::V4;
  }
  else if (bodyVersionString == "V3.3")
  {
    naoInfo_.bodyVersion = NaoVersion::V3_3;
  }
  else
  {
    naoInfo_.bodyVersion = NaoVersion::UNKNOWN;
  }

  if (headVersionString == "6.0.0")
  {
    naoInfo_.headVersion = NaoVersion::V6;
  }
  else if (headVersionString == "V5.0")
  {
    naoInfo_.headVersion = NaoVersion::V5;
  }
  else if (headVersionString == "V4.0")
  {
    naoInfo_.headVersion = NaoVersion::V4;
  }
  else if (headVersionString == "V3.3")
  {
    naoInfo_.headVersion = NaoVersion::V3_3;
  }
  else
  {
    naoInfo_.headVersion = NaoVersion::UNKNOWN;
  }

  // Export the NaoInfo to provide it in tuhhSDK.base for Export Diff functionality in MATE
  Uni::Value value = Uni::Value(Uni::ValueType::OBJECT);
  value << naoInfo_;
  config.set("tuhhSDK.base", "NaoInfo", value);
}

void NaoInterface::extractJoints(msgpack::object& array,
                                 std::array<float, keys::joints::JOINTS_MAX>& jointData)
{
  for (std::size_t i = 0; i < array.via.array.size; i++)
  {
    jointData[jointsRemapping_[i]] = array.via.array.ptr[i].as<float>();
  }
  jointData[keys::joints::R_HIP_YAW_PITCH] = jointData[keys::joints::L_HIP_YAW_PITCH];
}

void NaoInterface::extractVector2(msgpack::object& array, float* dest)
{
  for (int i = 0; i < 2; ++i)
  {
    dest[i] = array.via.array.ptr[i].as<float>();
  }
}

void NaoInterface::extractVector3(msgpack::object& array, float* dest)
{
  for (int i = 0; i < 3; ++i)
  {
    dest[i] = array.via.array.ptr[i].as<float>();
  }
}

void NaoInterface::extractFSRs(msgpack::object& array,
                               std::array<float, keys::sensor::FSR_MAX>& leftFSR,
                               std::array<float, keys::sensor::FSR_MAX>& rightFSR)
{
  for (int i = 0; i < 4; ++i)
  {
    leftFSR[i] = array.via.array.ptr[i].as<float>();
  }
  for (int i = 4; i < 8; ++i)
  {
    rightFSR[i - 4] = array.via.array.ptr[i].as<float>();
  }
}

void NaoInterface::extractBattery(msgpack::object& array,
                                  std::array<float, keys::sensor::BATTERY_MAX>& battery)
{
  for (int i = 0; i < keys::sensor::BATTERY_MAX; ++i)
  {
    battery[batteryRemapping_[i]] = array.via.array.ptr[i].as<float>();
  }
}

void NaoInterface::extractSwitches(msgpack::object& array,
                                   std::array<float, keys::sensor::SWITCH_MAX>& switches)
{
  for (int i = 0; i < keys::sensor::SWITCH_MAX; ++i)
  {
    switches[switchesRemapping_[i]] = array.via.array.ptr[i].as<float>();
  }
}

void NaoInterface::extractSonar(msgpack::object& array,
                                std::array<float, keys::sensor::SONAR_MAX>& sonar)
{
  sonar[keys::sensor::SONAR_LEFT_SENSOR_0] = array.via.array.ptr[0].as<float>();
  sonar[keys::sensor::SONAR_RIGHT_SENSOR_0] = array.via.array.ptr[1].as<float>();
}

void NaoInterface::packJoints(msgpack::packer<msgpack::sbuffer>& packer,
                              std::array<float, keys::joints::JOINTS_MAX>& jointData,
                              std::string name)
{
  packer.pack(name);
  packer.pack_array(jointsRemapping_.size());
  for (std::size_t i = 0; i < jointsRemapping_.size(); ++i)
  {
    packer.pack_float(jointData[jointsRemapping_[i]]);
  }
}

void NaoInterface::packFloatArray(msgpack::packer<msgpack::sbuffer>& packer, float* src,
                                  std::vector<int>& remapping, std::string name)
{
  packer.pack(name);
  packer.pack_array(remapping.size());
  for (std::size_t i = 0; i < remapping.size(); ++i)
  {
    packer.pack_float(src[remapping[i]]);
  }
}
