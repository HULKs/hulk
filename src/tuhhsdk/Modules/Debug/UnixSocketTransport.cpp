#include "UnixSocketTransport.hpp"

#include <iostream>
#include <list>
#include <set>
#include <thread>
#include <unordered_map>

#include <boost/array.hpp>
#include <boost/asio.hpp>
#include <boost/filesystem.hpp>
// because boost::asio defines a macro called ERROR we can't use LogLevel::ERROR here directly
#include "Definitions/windows_definition_fix.hpp"

#include "Libs/json/json.h"
#include "Tools/Var/SpscQueue.hpp"
#include "print.h"

#include "Debug.h"
#include "DebugMessageFormat.h"
#include "JpegConverter.h"


#ifndef _WIN32

// Type definitions

typedef std::unordered_map<std::string, DebugData> DebugDataMap;
typedef SpscRing<std::string, 256> DebugQueue;
typedef std::unordered_map<std::string, DebugQueue> DebugQueueMap;


// =========================

class UnixSocketTransport::Impl
{
private:
  boost::asio::io_service ioService_;
  boost::asio::local::stream_protocol::endpoint serverEndpoint_;
  boost::asio::local::stream_protocol::acceptor acceptor_;
  boost::asio::local::stream_protocol::socket socket_;


  std::shared_ptr<std::thread> backgroundThread_;

  std::list<std::shared_ptr<UnixSocketTransport::Session>> sessions_;
  std::mutex listMutex_;
  Debug& debug_;

  void startAccept();

  DebugDataMap data_;
  DebugQueueMap queues_;
  std::set<std::string> imageKeys_;

  JpegConverter jpegConv_;

public:
  Impl(const std::string& file, Debug& debug);
  ~Impl();

  DebugDataMap& getData();
  std::set<std::string>& getImageKeys();

  void startBackgroundThread();
  void connected(std::shared_ptr<UnixSocketTransport::Session> session);
  void disconnected(std::shared_ptr<UnixSocketTransport::Session> session);

  void sendImage(const std::string& key, const Image& img);

  void update(const DebugData& data);
  void pushQueue(const std::string& key, const std::string& message);
  void transport();
};

// =========================

class UnixSocketTransport::Session : public std::enable_shared_from_this<UnixSocketTransport::Session>
{
private:
  Debug& debug_;
  UnixSocketTransport::Impl& server_;
  boost::asio::local::stream_protocol::socket socket_;

  DebugMessageHeader header_;
  std::vector<char> bodyBuffer_;

  std::set<std::string> subscriptionList_;

  void readHeader();
  void readBody();

  void parseBody(const std::string& body);
  void subscribe(const std::string& key);
  void subscribeBulk(const std::string& json);
  void unsubscribe(const std::string& key);
  void transmitList();

  std::atomic<bool> canTransmit_;

public:
  Session(Debug& debug, UnixSocketTransport::Impl& server, boost::asio::local::stream_protocol::socket socket);
  ~Session();
  void start();
  void sendImage(const std::string& key, uint16_t width, uint16_t height, uint32_t imgSize, SharedCVData imgData);
  void transport();
};

//================================
// UnixSocketTransportSession
//================================

UnixSocketTransport::Session::Session(Debug& debug, UnixSocketTransport::Impl& server, boost::asio::local::stream_protocol::socket socket)
  : debug_(debug)
  , server_(server)
  , socket_(std::move(socket))
  , bodyBuffer_()
  , subscriptionList_()
  , canTransmit_(true)
{
}

UnixSocketTransport::Session::~Session()
{
  for (auto& key : subscriptionList_)
  {
    debug_.unsubscribe(key);
  }
}

void UnixSocketTransport::Session::start()
{
  server_.connected(shared_from_this());
  readHeader();
}

void UnixSocketTransport::Session::readHeader()
{
  auto self(shared_from_this());
  boost::asio::async_read(socket_, boost::asio::buffer(&header_, sizeof(DebugMessageHeader)),
                          [this, self](boost::system::error_code error, std::size_t length) {
                            if (error)
                            {
                              if (!(error == boost::asio::error::eof && length != sizeof(DebugMessageHeader)))
                              {
                                Log(LogLevel::WARNING) << "UnixSocketTransport: error while receiving header, disconnecting. Error: " << error.message();
                                server_.disconnected(self);
                                return;
                              }
                            }

                            Log(LogLevel::DEBUG) << "UnixSocketTransport: received header";

                            readBody();
                          });
}

void UnixSocketTransport::Session::readBody()
{
  bodyBuffer_.resize(header_.msgLength);

  auto self(shared_from_this());
  boost::asio::async_read(socket_, boost::asio::buffer(bodyBuffer_), [this, self](boost::system::error_code error, std::size_t /*length*/) {
    if (error)
    {
      Log(LogLevel::WARNING) << "UnixSocketTransport: error while receiving body, disconnecting. Error: " << error.message();
      server_.disconnected(self);
      return;
    }

    Log(LogLevel::INFO) << "UnixSocketTransport: received body";

    std::string body(bodyBuffer_.begin(), bodyBuffer_.end());
    parseBody(body);

    Log(LogLevel::DEBUG) << "UnixSocketTransport: Parsed Body!";

    readHeader();
  });
}

void UnixSocketTransport::Session::parseBody(const std::string& body)
{
  switch (header_.msgType)
  {
    case DM_SUBSCRIBE:
    {
      Log(LogLevel::DEBUG) << "DM_SUBSCRIBE-Message received: " + body;
      subscribe(body);
    }
    break;
    case DM_SUBSCRIBE_BULK:
    {
      Log(LogLevel::DEBUG) << "DM_SUBSCRIBE_BULK-Message received: " << body;
      subscribeBulk(body);
    }
    break;
    case DM_UNSUBSCRIBE:
    {
      Log(LogLevel::DEBUG) << "DM_UNSUBSCRIBE-Message received: " << body;
      unsubscribe(body);
    }
    break;
    case DM_REQUEST_LIST:
    {
      Log(LogLevel::DEBUG) << "DM_REQUEST_LIST-Message received.";
      transmitList();
    }
    break;
    default:
      Log(LogLevel::WARNING) << "Command is not to be implemented on the Server/Robot.";
      break;
  }
}

void UnixSocketTransport::Session::subscribe(const std::string& key)
{
  subscriptionList_.insert(key);
  debug_.subscribe(key);
}

void UnixSocketTransport::Session::unsubscribe(const std::string& key)
{
  subscriptionList_.erase(key);
  debug_.unsubscribe(key);
}

void UnixSocketTransport::Session::subscribeBulk(const std::string& json)
{
  Json::Reader reader;
  Json::Value root(Json::ValueType::objectValue);
  reader.parse(json, root, false);

  const Json::Value keys = root["keys"];
  for (uint32_t i = 0; i < keys.size(); ++i)
  {
    subscriptionList_.insert(keys[i].asString());
    debug_.subscribe(keys[i].asString());
  }
}

void UnixSocketTransport::Session::transmitList()
{
  DebugDataMap& data = server_.getData();

  Uni::Value root(Uni::ValueType::OBJECT);
  Uni::Value uniArray(Uni::ValueType::ARRAY);

  int i = 0;
  for (auto it = data.begin(); it != data.end(); ++it)
  {
    uniArray[i++] << it->second;
  }

  auto images = server_.getImageKeys();
  for (auto it = images.begin(); it != images.end(); ++it)
  {
    Uni::Value entry(Uni::ValueType::OBJECT);
    entry["key"] << *it;
    entry["isImage"] << true;
    uniArray[i++] = entry;
  }

  root["keys"] = uniArray;

  std::shared_ptr<std::string> json = std::make_shared<std::string>(Uni::Converter::toJsonString(root, false));

  DebugMessageHeader hdr;
  hdr.msgType = DM_LIST;
  hdr.msgLength = json->length();

  std::vector<boost::asio::const_buffer> sendBuffers;

  sendBuffers.push_back(boost::asio::buffer(&hdr, sizeof(DebugMessageHeader)));
  sendBuffers.push_back(boost::asio::buffer(*json));

  try
  {
    bool expected = true;
    if (canTransmit_.compare_exchange_weak(expected, false))
    {
      // send
      auto self(shared_from_this());
      boost::asio::async_write(socket_, sendBuffers, [this, self, json](boost::system::error_code error, std::size_t /*length*/) {
        canTransmit_.store(true);
        if (error)
        {
          Log(LogLevel::WARNING) << "UnixSocketTransport: error while sending List, disconnecting. Error: " << error.message();
          server_.disconnected(self);
          return;
        }

        Log(LogLevel::DEBUG) << "UnixSocketTransport: sent List.";
      });
    }
  }
  catch (boost::system::system_error&)
  {
    print("Error transmitting debug updates!", LogLevel::ERROR);
  }
}

void UnixSocketTransport::Session::transport()
{
  try
  {
    DebugDataMap& data = server_.getData();

    // return early, when nothing to transmit.
    if (subscriptionList_.empty())
      return;

    // Serialize all the keys, they subscribed to, but nothing more.
    Uni::Value root(Uni::ValueType::ARRAY);
    int i = 0;
    for (auto key = subscriptionList_.begin(); key != subscriptionList_.end(); ++key)
    {
      auto it = data.find(*key);
      if (it != data.end())
      {
        it->second.toValue(root[i++]);
      }
    }

    // if array is empty, return more or less early.
    if (i == 0)
      return;

    std::shared_ptr<std::string> json = std::make_shared<std::string>(Uni::Converter::toJsonString(root, false));

    // prepare the buffers.
    std::vector<boost::asio::const_buffer> sendBuffers;

    // Create a Message Header containing the length of the json etc.
    DebugMessageHeader header;
    header.msgLength = json->length();
    header.msgType = DM_UPDATE;

    // concatenate the buffers.
    sendBuffers.push_back(boost::asio::buffer(&header, sizeof(DebugMessageHeader)));
    sendBuffers.push_back(boost::asio::buffer(*json));

    try
    {
      // check if sending an image right now...
      bool expected = true;
      if (canTransmit_.compare_exchange_weak(expected, false))
      {
        // and send that stuff to the subscriber
        auto self(shared_from_this());
        boost::asio::async_write(socket_, sendBuffers, [this, self, header, json](boost::system::error_code error, std::size_t /*length*/) {
          canTransmit_.store(true);
          if (error)
          {
            Log(LogLevel::WARNING) << "UnixSocketTransport: error while sending Updates, disconnecting. Error: " << error.message();
            server_.disconnected(self);
            return;
          }

          // Log(LogLevel::DEBUG) <<  "UnixSocketTransport: sent Updates.";
        });
      }
    }
    catch (boost::system::system_error&)
    {
      Log(LogLevel::ERROR) << "Error transmitting debug updates!";
    }
  }
  catch (std::exception& e)
  {
    Log(LogLevel::ERROR) << "Exception transmitting debug updates: " << e.what();
  }
}

void UnixSocketTransport::Session::sendImage(const std::string& key, uint16_t width, uint16_t height, uint32_t imgSize, SharedCVData imgData)
{
  if (subscriptionList_.find(key) == subscriptionList_.end())
  {
    // no subscription.
    return;
  }

  uint16_t length = key.length();
  std::shared_ptr<std::string> k = std::make_shared<std::string>(key);

  // create the header
  DebugMessageHeader header;
  header.msgType = DM_IMAGE;
  header.msgLength = sizeof(width) + sizeof(height) + sizeof(length) + length + imgSize;

  // combine header, image size and image data into buffers vector
  std::vector<boost::asio::const_buffer> buffers;
  buffers.push_back(boost::asio::buffer(&header, sizeof(DebugMessageHeader)));
  buffers.push_back(boost::asio::buffer(&width, sizeof(width)));
  buffers.push_back(boost::asio::buffer(&height, sizeof(height)));
  buffers.push_back(boost::asio::buffer(&length, sizeof(length)));
  buffers.push_back(boost::asio::buffer(*k, length));
  buffers.push_back(boost::asio::buffer(*imgData, imgSize));

  // send
  auto self(shared_from_this());
  bool expected = true;
  while (!canTransmit_.compare_exchange_strong(expected, false))
  {
    std::this_thread::yield();
    std::this_thread::sleep_for(std::chrono::microseconds(10));
    expected = true;
  }
  boost::asio::async_write(socket_, buffers, [this, self, header, width, height, length, k, imgData](boost::system::error_code error, std::size_t /*length*/) {
    canTransmit_.store(true);
    if (error)
    {
      Log(LogLevel::WARNING) << "UnixSocketTransport: error while sending image, disconnecting. Error: " << error.message();
      server_.disconnected(self);
      return;
    }
  });
}


//================================
// pimpl
//================================
UnixSocketTransport::Impl::Impl(const std::string& file, Debug& debug)
  : ioService_()
  , serverEndpoint_(file)
  , acceptor_(ioService_, serverEndpoint_)
  , socket_(ioService_)
  , debug_(debug)
  , imageKeys_()
{
  startAccept();
}

UnixSocketTransport::Impl::~Impl()
{
  ioService_.stop();
  backgroundThread_->join();
  socket_.close();
}

DebugDataMap& UnixSocketTransport::Impl::getData()
{
  return data_;
}

std::set<std::string>& UnixSocketTransport::Impl::getImageKeys()
{
  return imageKeys_;
}

void UnixSocketTransport::Impl::startBackgroundThread()
{
  backgroundThread_ = std::make_shared<std::thread>([this]() {
    Log(LogLevel::INFO) << "UnixSocketTransport: Starting background thread.";
    ioService_.run();
    Log(LogLevel::INFO) << "UnixSocketTransport: background thread terminated.";
  });
}

void UnixSocketTransport::Impl::startAccept()
{
  Log(LogLevel::INFO) << "UnixSocketTransport: Waiting for connection";
  acceptor_.async_accept(socket_, [this](const boost::system::error_code& error) {
    if (!error)
    {
      std::make_shared<UnixSocketTransport::Session>(debug_, *this, std::move(socket_))->start();
    }

    startAccept();
  });
}

void UnixSocketTransport::Impl::connected(std::shared_ptr<UnixSocketTransport::Session> session)
{
  std::lock_guard<std::mutex> lg(listMutex_);
  sessions_.push_back(session);
}

void UnixSocketTransport::Impl::disconnected(std::shared_ptr<UnixSocketTransport::Session> session)
{
  std::lock_guard<std::mutex> lg(listMutex_);
  sessions_.remove(session);
}


void UnixSocketTransport::Impl::sendImage(const std::string& key, const Image& img)
{
  auto imgData = jpegConv_.convert(img);

  imageKeys_.insert(key);

  std::lock_guard<std::mutex> lg(listMutex_);
  for (auto it = sessions_.begin(); it != sessions_.end(); it++)
  {
    (*it)->sendImage(key, img.size_.x(), img.size_.y(), imgData->size(), imgData);
  }
}

void UnixSocketTransport::Impl::update(const DebugData& data)
{
  data_[data.key] = data;
}

void UnixSocketTransport::Impl::pushQueue(const std::string& key, const std::string& message)
{
  queues_[key].push(message);
}

void UnixSocketTransport::Impl::transport()
{
  for (auto it = queues_.begin(); it != queues_.end(); ++it)
  {
    DebugData data(it->first);
    std::string lastMessage;

    int i = 0;
    while (it->second.pop(lastMessage))
    {
      data.value[i++] << lastMessage;
    }

    data_[it->first] = data;
  }

  std::lock_guard<std::mutex> lg(listMutex_);
  for (auto it = sessions_.begin(); it != sessions_.end(); it++)
  {
    (*it)->transport(); // TODO: possible sigsegv here
  }
}

//================================
// UnixSocketTransport
//================================
UnixSocketTransport::UnixSocketTransport(const std::string& file, Debug& debug)
{
  std::remove(file.c_str());
  boost::filesystem::create_directories(boost::filesystem::path(file).parent_path());
  pimpl_.reset(new Impl(file, debug));
  pimpl_->startBackgroundThread();
}

UnixSocketTransport::~UnixSocketTransport() {}

void UnixSocketTransport::sendImage(const std::string& key, const Image& img)
{
  pimpl_->sendImage(key, img);
}

void UnixSocketTransport::update(const DebugData& data)
{
  pimpl_->update(data);
}

void UnixSocketTransport::pushQueue(const std::string& key, const std::string& message)
{
  pimpl_->pushQueue(key, message);
}

void UnixSocketTransport::transport()
{
  pimpl_->transport();
}

#endif
