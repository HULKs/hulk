#include <boost/interprocess/sync/scoped_lock.hpp>

#include <chrono>
#include <thread>

#include "Tools/Time.hpp"
#include "print.h"
#include "Tools/Time.hpp"
#include "Modules/Configuration/Configuration.h"

#include "NaoInterface.hpp"
#include "Hardware/Nao/SMO.h"


NaoInterface::NaoInterface() :
  topCamera_(Camera::TOP),
  bottomCamera_(Camera::BOTTOM),
  audioInterface_(),
  currentCamera_(Camera::TOP)
{
  // When tuhhNao has been started directly after naoqi, libtuhhALModule will not be started and the shared memory will not exist.
  // Thus, this process needs to retry opening.
  unsigned int tries = 10;
  while (true)
  {
    try
    {
      tuhhprint::print("Try to open sharedMemory: ", SMO::shmName, LogLevel::INFO);
      segment_ = boost::interprocess::shared_memory_object(boost::interprocess::open_only, SMO::shmName, boost::interprocess::read_write);
    }
    catch  (const boost::interprocess::interprocess_exception& e)
    {
      if (!(--tries))
      {
        throw;
      }
      std::this_thread::sleep_for(std::chrono::seconds(1));
      continue;
    }
    break;
  }
  try
  {
    region_ = boost::interprocess::mapped_region(segment_, boost::interprocess::read_write);
    shmBlock_ = reinterpret_cast<SharedBlock*>(region_.get_address());
    tuhhprint::print("... done opening sharedMemory.", LogLevel::INFO);

    {
      shmBlock_->semaphore.wait();
      // wait for first sensor data to make sure naoqi is there and get nao info
      boost::interprocess::scoped_lock<SharedBlock::mutex_t> lock(shmBlock_->accessMutex);
      rawInfo_ = shmBlock_->naoInfoKey;
    }
  }
  catch (boost::interprocess::interprocess_exception &e)
  {
    tuhhprint::print("NaoInterface::NaoInterface: Caught boost::interprocess::interprocess_exception: ", LogLevel::ERROR);
    tuhhprint::print(e.what(), LogLevel::ERROR);
    throw;
  }
  catch ( ... )
  {
    tuhhprint::print("Unknown exception in NaoInterface::NaoInterface()", LogLevel::ERROR);
    throw;
  }
}

NaoInterface::~NaoInterface()
{
}

void NaoInterface::configure(Configuration& config)
{
  // This needs to be done here because now the identity of the NAO is known.
  topCamera_.configure(config);
  bottomCamera_.configure(config);
}

void NaoInterface::setJointAngles(const std::vector<float>& angles)
{
  assert(angles.size() == shmBlock_->commandAngles.size());
  boost::interprocess::scoped_lock<SharedBlock::mutex_t> lock(shmBlock_->accessMutex);
  for (unsigned int i = 0; i < angles.size(); i++) {
    shmBlock_->commandAngles[i] = angles[i];
  }
  shmBlock_->newAngles = true;
}

void NaoInterface::setJointStiffnesses(const std::vector<float>& stiffnesses)
{
  assert(stiffnesses.size() == shmBlock_->commandStiffnesses.size());
  boost::interprocess::scoped_lock<SharedBlock::mutex_t> lock(shmBlock_->accessMutex);
  for (unsigned int i = 0; i < stiffnesses.size(); i++) {
    shmBlock_->commandStiffnesses[i] = stiffnesses[i];
  }
  shmBlock_->newStiffnesses = true;
}

void NaoInterface::setLEDs(const std::vector<float>& leds)
{
  assert(leds.size() == shmBlock_->commandLEDs.size());
  boost::interprocess::scoped_lock<SharedBlock::mutex_t> lock(shmBlock_->accessMutex);
  for (unsigned int i = 0; i < leds.size(); i++) {
    shmBlock_->commandLEDs[i] = leds[i];
  }
  shmBlock_->newLEDs = true;
}

void NaoInterface::setSonar(const float sonar)
{
  boost::interprocess::scoped_lock<SharedBlock::mutex_t> lock(shmBlock_->accessMutex);
  shmBlock_->commandSonar = sonar;
  shmBlock_->newSonar = true;
}

void NaoInterface::waitAndReadSensorData(NaoSensorData& data)
{
  TimePoint start = TimePoint::getCurrentTime();
  shmBlock_->semaphore.wait();
  boost::interprocess::scoped_lock<SharedBlock::mutex_t> lock(shmBlock_->accessMutex);
  print("NaoInterface::waitAndReadSensorData() -- time waiting and reading shared memory (ms):\t", getTimeDiff(TimePoint::getCurrentTime(), start, TDT::MILS), LogLevel::DEBUG);
  std::uint64_t readingStart = getThreadTime();
  try
  {
    // Joints
    {
      std::uint64_t start = getThreadTime();
      data.jointSensor = shmBlock_->jointSensor;
      data.jointCurrent = shmBlock_->jointCurrent;
      data.jointTemperature = shmBlock_->jointTemperature;
      data.jointStatus = shmBlock_->jointStatus;
      print("NaoInterface::waitAndReadSensorData() -- time updating joints (ms):\t", static_cast<float>(getThreadTime() - start) / 1000000, LogLevel::DEBUG);
    }

    // Buttons
    {
      std::uint64_t start = getThreadTime();
      data.switches = shmBlock_->switches;
      print("NaoInterface::waitAndReadSensorData() -- time updating switches (ms):\t", static_cast<float>(getThreadTime() - start) / 1000000, LogLevel::DEBUG);
    }

    // IMU
    {
      std::uint64_t start = getThreadTime();
      data.imu = shmBlock_->imu;
      print("NaoInterface::waitAndReadSensorData() -- time updating IMU (ms):\t", static_cast<float>(getThreadTime() - start) / 1000000, LogLevel::DEBUG);
    }

    // FSR
    {
      std::uint64_t start = getThreadTime();
      data.fsrLeft = shmBlock_->fsrLeft;
      data.fsrRight = shmBlock_->fsrRight;
      print("NaoInterface::waitAndReadSensorData() -- time updating FSR (ms):\t", static_cast<float>(getThreadTime() - start) / 1000000, LogLevel::DEBUG);
    }

    // Sonar
    {
      std::uint64_t start = getThreadTime();
      data.sonar = shmBlock_->sonar;
      print("NaoInterface::waitAndReadSensorData() -- time updating sonar (ms):\t", static_cast<float>(getThreadTime() - start) / 1000000, LogLevel::DEBUG);
    }

    // Battery
    {
      std::uint64_t start = getThreadTime();
      data.battery = shmBlock_->battery;
      print("NaoInterface::waitAndReadSensorData() -- time updating battery (ms):\t", static_cast<float>(getThreadTime() - start) / 1000000, LogLevel::DEBUG);
    }

    // Time
    {
      std::uint64_t start = getThreadTime();
      const int64_t millisecondsSince1970 = std::chrono::duration<int64_t, std::nano>(shmBlock_->time).count() / 1000000;
      data.time = TimePoint(millisecondsSince1970 - TimePoint::getBaseTime());
      print("NaoInterface::waitAndReadSensorData() -- time updating time (ms):\t", static_cast<float>(getThreadTime() - start) / 1000000, LogLevel::DEBUG);
    }

    // Callback
    {
      std::uint64_t start = getThreadTime();
      if (shmBlock_->chestButtonPressed) {
        data.buttonCallbackList.push_back(CE_CHESTBUTTON_SIMPLE);
      }
      if (shmBlock_->chestButtonDoublePressed) {
        data.buttonCallbackList.push_back(CE_CHESTBUTTON_DOUBLE);
      }
      print("NaoInterface::waitAndReadSensorData() -- time updating callbacks(ms):\t", static_cast<float>(getThreadTime() - start) / 1000000, LogLevel::DEBUG);
    }

    //tuhhprint::print("readSharedMemory() -> finished reading from shmBlockSensor_", LogLevel::DEBUG);
  }
  catch (boost::interprocess::interprocess_exception &e)
  {
    tuhhprint::print("NaoInterface::waitAndReadSensorData(): boost::interprocess::interprocess_exception: ", LogLevel::ERROR);
    tuhhprint::print(e.what(), LogLevel::ERROR);
  }
  catch ( ... )
  {
    tuhhprint::print("Unknown exception in NaoInterface::waitAndReadSensorData()", LogLevel::ERROR);
  }
  print("NaoInterface::waitAndReadSensorData() -- total time updating (ms):\t", static_cast<float>(getThreadTime() - readingStart) / 1000000, LogLevel::DEBUG);
}

std::string NaoInterface::getFileRoot()
{
  return "/home/nao/naoqi/";
}

void NaoInterface::getNaoInfo(Configuration& config, NaoInfo& info)
{
  // This method is normally called only once, just to make sure...
  if (naoInfo_.bodyName.empty()) {
    initNaoInfo(config);
  }
  info = naoInfo_;
}

CameraInterface& NaoInterface::getCamera(const Camera camera)
{
  return (camera == Camera::TOP) ? topCamera_ : bottomCamera_;
}

CameraInterface& NaoInterface::getCurrentCamera()
{
  if (currentCamera_ == Camera::TOP)
  {
    // switch to bottom camera for next frame
    currentCamera_ = Camera::BOTTOM;
    return topCamera_;
  }
  else
  {
    currentCamera_ = Camera::TOP;
    return bottomCamera_;
  }
}

AudioInterface& NaoInterface::getAudio()
{
  return audioInterface_;
}

void NaoInterface::initNaoInfo(Configuration& config)
{
  print("NaoInterface::initNaoInfo", LogLevel::INFO);
  std::string bodyname = "";
  std::string headname = "";

  /// ID Mapping

  std::string bodyIDstr = rawInfo_[keys::naoinfos::BODY_ID];
  std::string headIDstr = rawInfo_[keys::naoinfos::HEAD_ID];

  config.mount("NaoInterface.id_map", "id_map.json", ConfigurationType::HEAD);
  Uni::Value& val = config.get("NaoInterface.id_map", "idmap.nao");

  try {
    for (auto it = val.listBegin(); it != val.listEnd(); it++) {
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
    print(std::string("Error when finding out my identity. (NaoInterface::initNaoInfo). ") + err.what(), LogLevel::ERROR);
  }

  if (bodyname.empty()) {
    print("body could not be identified",  LogLevel::ERROR);
  }
  if (headname.empty()) {
    print("head could not be identified",  LogLevel::ERROR);
  }
  if (bodyname.empty() || headname.empty()) {
    throw std::runtime_error("Could not determine either body or headname");
  }

  naoInfo_.bodyName = bodyname;
  naoInfo_.headName = headname;

  // Determine version
  std::string bodyVersionString = rawInfo_[keys::naoinfos::BODY_BASE_VERSION];
  std::string headVersionString = rawInfo_[keys::naoinfos::HEAD_BASE_VERSION];
  if (bodyVersionString == "V5.0") {
    naoInfo_.bodyVersion = NaoVersion::V5;
  } else if (bodyVersionString == "V4.0") {
    naoInfo_.bodyVersion = NaoVersion::V4;
  } else if (bodyVersionString == "V3.3") {
    naoInfo_.bodyVersion = NaoVersion::V3_3;
  } else {
    naoInfo_.bodyVersion = NaoVersion::UNKNOWN;
  }
  if (headVersionString == "V5.0") {
    naoInfo_.headVersion = NaoVersion::V5;
  } else if (headVersionString == "V4.0") {
    naoInfo_.headVersion = NaoVersion::V4;
  } else if (headVersionString == "V3.3") {
    naoInfo_.headVersion = NaoVersion::V3_3;
  } else {
    naoInfo_.headVersion = NaoVersion::UNKNOWN;
  }
}
