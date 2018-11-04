#include "UnixSocketTransport.hpp"

#include "Debug.h"
#include "DebugMessageFormat.h"
#include "JpegConverter.h"

#include "Libs/json/json.h"

#include <boost/array.hpp>
#include <boost/asio.hpp>
#include <boost/filesystem.hpp>
#include <boost/range.hpp>
// because boost::asio defines a macro called ERROR we can't use LogLevel::ERROR here directly
#include "Definitions/windows_definition_fix.hpp"

#include <iostream>
#include <list>
#include <numeric>
#include <set>
#include <thread>
#include <unordered_map>

#include "print.h"


#ifndef _WIN32

/**
 * @brief the implementation of the UnixSocketTransport class
 */
class UnixSocketTransport::Impl
{
public:
  /**
   * @brief Impl initializes members
   * @param file the file to use for sending debug data
   * @param debug a reference to debug (to get the transportable debugMap from)
   */
  Impl(const std::string& file, Debug& debug);
  /**
   * @brief destructor
   */
  ~Impl();
  /**
   * @brief startBackgroundThread starts the underlying ioService thread.
   */
  void startBackgroundThread();
  /**
   * @brief connected is called when a new client connects
   * @param session the new session
   */
  void connected(std::shared_ptr<UnixSocketTransport::Session> session);
  /**
   * @brief disconnected is called when a client disconnects
   * @param session the session to remove
   */
  void disconnected(std::shared_ptr<UnixSocketTransport::Session> session);
  /**
   * @brief hook that should be called after a debug source's cycle to transport the debug data.
   */
  void transport();

private:
  /// the ioService (background thread for sending debug data)
  boost::asio::io_service ioService_;
  /// the unix socket description to listen on
  boost::asio::local::stream_protocol::endpoint serverEndpoint_;
  /// acceptor provides async-waiting for new clients to connect
  boost::asio::local::stream_protocol::acceptor acceptor_;
  /// the unix socket to send/receive data on
  boost::asio::local::stream_protocol::socket socket_;

  /// pointer to this thread (running the ioService.run())
  std::shared_ptr<std::thread> backgroundThread_;

  /// list of all sessions
  std::list<std::shared_ptr<UnixSocketTransport::Session>> sessions_;
  /// mutex for the sessions list
  std::mutex listMutex_;
  /// reference to the debug instance.
  Debug& debug_;

  /**
   * @brief startAccept starts listening for new connections and handles them
   */
  void startAccept();
};


/**
 * @brief class to encapsulate single client sessions
 */
class UnixSocketTransport::Session
  : public std::enable_shared_from_this<UnixSocketTransport::Session>
{
public:
  /**
   * @brief Session initializes members
   * @param debug reference to the debug instance to get the debugMap from
   * @param server the server that is responsible for this session
   * @param socket the socket on which to send / receive data
   */
  Session(Debug& debug, UnixSocketTransport::Impl& server,
          boost::asio::local::stream_protocol::socket socket);
  /**
   * @brief destructor
   */
  ~Session();
  /**
   * @brief start starts this session
   */
  void start();
  /**
   * @brief hook that should be called after a debug source's cycle to transport the debug data.
   */
  void transport();

private:
  /**
   * @brief readHeader reads / parses a debug header
   */
  void readHeader();
  /**
   * @brief readBody reads a debug message's body
   */
  void readBody();

  /**
   * @brief parseBody parses a debug message's body
   * @param body the body to parse
   */
  void parseBody(const std::string& body);
  /**
   * @brief subscribe subscribes the given key.
   * Multiple subscriptions are allowed. It is ensured that a key stayes subscribed until
   * unsubscribe is called as often as a key was subscribed.
   * @param key the key to subscribe
   */
  void subscribe(const std::string& key);
  /**
   * @brief subscribeBulk subscribes all given keys (json)
   * Look at subscribe() for more details
   * @param json the list of keys to subscribe
   */
  void subscribeBulk(const std::string& json);
  /**
   * @brief unsubscribe unsubscribes the given key.
   * @param key The key to unsubscribe
   */
  void unsubscribe(const std::string& key);
  /**
   * @brief transmitList sends a list of all keys that are available.
   * @return bool; true if the list was transmitted successfully
   */
  bool transmitList();

  /**
   * @brief ImageData contains the compressed image
   */
  struct ImageData
  {
    /// the debug message header
    DebugMessageHeader hdr;
    /// the debug key
    std::string key;
    /// image width given in pixels
    uint16_t width;
    /// image height given in pixels
    uint16_t height;
    /// the length of this image data (the whole struct) (given in bytes)
    uint16_t length;
    /// the actual image data (compressed image)
    CVData data;
  };

  /// reference to the debug instance to get the debugMap from
  Debug& debug_;
  /// the server that is responsible for this session
  UnixSocketTransport::Impl& server_;
  /// the socket on which to send / receive data
  boost::asio::local::stream_protocol::socket socket_;

  /// the received debug message header
  DebugMessageHeader receivedHeader_;
  /// a buffer for message bodies
  std::vector<char> bodyBuffer_;

  /// a list if subscribed keys
  std::set<std::string> subscriptionList_;
  /// a mutex to read/write access for the keys
  std::mutex subscriptionListMutex_;

  /// the json data string that was received / will be sent.
  std::string jsonData_;
  /// the debug message header to send.
  DebugMessageHeader headerToSend_;
  /// a map of key-image pairs containing pointers to all images to send
  std::unordered_map<std::string, std::unique_ptr<ImageData>> imagesToSend_;

  /// if it is safe to transmit data
  std::atomic<bool> canTransmit_;
  /// if the key list was requested by the client
  std::atomic<bool> sendList_;

  /// the sendBuffers for async write
  std::vector<boost::asio::const_buffer> sendBuffers_;
  /// the actual list of all keys to send on request
  Uni::Value transmitList_;
  /// a map for tracking the status of the serialization of the module manager key lists.
  std::map<std::string, bool> serializedMMList_;

  /// the jpeg converter
  JpegConverter jpegConv_;

#ifdef ITTNOTIFY_FOUND
  /// an event to call while sending data with asio (vtune instrumentation)
  __itt_event eventTransmission_;
#endif
};

//================================
// UnixSocketDebugSession
//================================

UnixSocketTransport::Session::Session(Debug& debug, UnixSocketTransport::Impl& server,
                                      boost::asio::local::stream_protocol::socket socket)
  : debug_(debug)
  , server_(server)
  , socket_(std::move(socket))
  , canTransmit_(true)
  , sendList_(false)
  , transmitList_(Uni::ValueType::ARRAY)
{
#ifdef ITTNOTIFY_FOUND
  eventTransmission_ = __itt_event_create("Transmission", 12);
#endif
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
  std::lock_guard<std::mutex> lg(subscriptionListMutex_);
  server_.connected(shared_from_this());
  readHeader();
}

void UnixSocketTransport::Session::readHeader()
{
  auto self(shared_from_this());
  boost::asio::async_read(
      socket_, boost::asio::buffer(&receivedHeader_, sizeof(DebugMessageHeader)),
      [this, self](boost::system::error_code error, std::size_t length) {
        if (error)
        {
          if (!(error == boost::asio::error::eof && length != sizeof(DebugMessageHeader)))
          {
            Log(LogLevel::ERROR)
                << "UnixSocketTransport: error while receiving header, disconnecting. Error: "
                << error.message();
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
  bodyBuffer_.resize(receivedHeader_.msgLength);

  auto self(shared_from_this());
  boost::asio::async_read(
      socket_, boost::asio::buffer(bodyBuffer_),
      [this, self](boost::system::error_code error, std::size_t /*length*/) {
        if (error)
        {
          Log(LogLevel::ERROR)
              << "UnixSocketTransport: error while receiving body, disconnecting. Error: "
              << error.message();
          server_.disconnected(self);
          return;
        }

        Log(LogLevel::DEBUG) << "UnixSocketTransport: received body";

        std::string body(bodyBuffer_.begin(), bodyBuffer_.end());
        parseBody(body);

        Log(LogLevel::DEBUG) << "UnixSocketTransport: Parsed Body!";

        readHeader();
      });
}

void UnixSocketTransport::Session::parseBody(const std::string& body)
{
  switch (receivedHeader_.msgType)
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
      sendList_.store(true);
    }
    break;
    default:
      Log(LogLevel::WARNING) << "Command is not to be implemented on the Server/Robot.";
      break;
  }
}

void UnixSocketTransport::Session::subscribe(const std::string& key)
{
  std::lock_guard<std::mutex> lg(subscriptionListMutex_);
  subscriptionList_.insert(key);
  debug_.subscribe(key);
}

void UnixSocketTransport::Session::unsubscribe(const std::string& key)
{
  std::lock_guard<std::mutex> lg(subscriptionListMutex_);
  subscriptionList_.erase(key);
  debug_.unsubscribe(key);
}

void UnixSocketTransport::Session::subscribeBulk(const std::string& json)
{
  std::lock_guard<std::mutex> lg(subscriptionListMutex_);
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

bool UnixSocketTransport::Session::transmitList()
{
  auto& data = debug_.getDebugSources();

  Uni::Value root(Uni::ValueType::OBJECT);
  Uni::Value imageEntry(Uni::ValueType::OBJECT);

  int i = transmitList_.size();
  for (const auto& debugSource : data)
  {
    DebugDatabase::DebugMap* debugMap = debugSource.second.currentDebugMap;
    auto it = serializedMMList_.find(debugSource.first);
    if (it == serializedMMList_.end())
    {
      serializedMMList_[debugSource.first] = false;
    }
    else if (serializedMMList_[debugSource.first])
    {
      continue;
    }

    if (debugMap == nullptr)
    {
      Log(LogLevel::INFO) << "Unable to compute the complete keylist, waiting for next cycle.";
      continue;
    }

    serializedMMList_[debugSource.first] = true;


    for (const auto& dataEntry : debugMap->getDebugMap())
    {
      if (!dataEntry.second.isImage)
      {
        DebugData dat(dataEntry.first, dataEntry.second.data.get());
        transmitList_[i] << dat;
      }
      else
      {
        imageEntry["key"] << dataEntry.first;
        imageEntry["isImage"] << true;
        transmitList_[i] = imageEntry;
      }
      i++;
    }
  }

  bool allSerialized = std::accumulate(
      serializedMMList_.begin(), serializedMMList_.end(), true,
      [](const auto& current, const auto& it) -> bool { return current && it.second; });

  if (!allSerialized)
  {
    return false;
  }

  root["keys"] = transmitList_;

  std::shared_ptr<std::string> json =
      std::make_shared<std::string>(Uni::Converter::toJsonString(root, false));

  std::shared_ptr<DebugMessageHeader> hdr = std::make_shared<DebugMessageHeader>();
  hdr->msgType = DM_LIST;
  hdr->msgLength = json->length();

  sendBuffers_.clear();
  sendBuffers_.push_back(boost::asio::buffer(hdr.get(), sizeof(DebugMessageHeader)));
  sendBuffers_.push_back(boost::asio::buffer(*json));

  try
  {
    bool expected = true;
    if (canTransmit_.compare_exchange_weak(expected, false))
    {
    // send
#ifdef NAO
      auto self(shared_from_this());
#ifdef ITTNOTIFY_FOUND
      __itt_event_start(this->eventTransmission_);
#endif
      boost::asio::async_write(
          socket_, boost::make_iterator_range(sendBuffers_.begin(), sendBuffers_.end()),
          [this, self, hdr, json](boost::system::error_code error, std::size_t /*length*/) {
            canTransmit_.store(true);
            if (error)
            {
              Log(LogLevel::ERROR)
                  << "UnixSocketTransport: error while sending List, disconnecting. Error: "
                  << error.message();
              server_.disconnected(self);
              return;
            }
#ifdef ITTNOTIFY_FOUND
            __itt_event_end(this->eventTransmission_);
#endif

            Log(LogLevel::DEBUG) << "UnixSocketTransport: sent List.";
          });
#else
#ifdef ITTNOTIFY_FOUND
      __itt_event_start(this->eventTransmission_);
#endif
      boost::asio::write(socket_,
                         boost::make_iterator_range(sendBuffers_.begin(), sendBuffers_.end()));
      canTransmit_.store(true);
#ifdef ITTNOTIFY_FOUND
      __itt_event_end(this->eventTransmission_);
#endif
#endif
    }
  }
  catch (boost::system::system_error&)
  {
    canTransmit_.store(true);
    print("Error transmitting debug updates!", LogLevel::ERROR);
  }

  return true;
}

void UnixSocketTransport::Session::transport()
{
  if (sendList_.load())
  {
    if (transmitList())
    {
      sendList_.store(false);
    }
  }
  else
  {
    transmitList_.clearList();
    serializedMMList_.clear();
  }

  // return early, when nothing to transmit.
  if (subscriptionList_.empty())
    return;

  // check if sending right now...
  bool expected = true;
  if (!canTransmit_.compare_exchange_weak(expected, false))
  {
    Log(LogLevel::DEBUG) << "Could not transmit debug updates, already transmitting";
    return;
  }

  try
  {
    const auto& debugSources = debug_.getDebugSources();

    // prepare the buffers.

    sendBuffers_.clear();
    jsonData_.clear();

    jsonData_ += "[";

    // Serialize all the keys, they subscribed to, but nothing more.
    bool isFirst = true;
    std::unique_lock<std::mutex> lg(subscriptionListMutex_);

    for (auto key = subscriptionList_.begin(); key != subscriptionList_.end(); ++key)
    {
      const DebugDatabase::DebugMapEntry* debugMapEntry = nullptr;
      for (const auto& debugSource : debugSources)
      {
        auto currentDebugMap = debugSource.second.currentDebugMap;
        if (currentDebugMap != nullptr)
        {
          auto it = currentDebugMap->getDebugMap().find(*key);
          if (it != currentDebugMap->getDebugMap().end() &&
              currentDebugMap->getUpdateTime() == it->second.updateTime)
          {
            debugMapEntry = &(it->second);
            break;
          }
        }
      }

      if (debugMapEntry == nullptr)
      {
        Log(LogLevel::DEBUG) << "Key might only be available in another debugSource!";
        continue;
      }

      if (debugMapEntry->data->type() == Uni::ValueType::NIL &&
          debugMapEntry->image->size_ == Vector2i::Zero())
      {
        continue;
      }

      if (!debugMapEntry->isImage)
      {
        DebugData dat(*key, debugMapEntry->data.get());
        Uni::Value value;
        value << dat;
        if (!isFirst)
        {
          jsonData_ += ",";
        }
        jsonData_ += Uni::Converter::toJsonString(value, false);
        isFirst = false;
      }
      else
      {
        auto imageDataIt = imagesToSend_.find(*key);
        if (imageDataIt == imagesToSend_.end())
        {
          std::unique_ptr<ImageData> data = std::make_unique<ImageData>();
          data->key = *key;
          imageDataIt = imagesToSend_.emplace(std::make_pair(*key, std::move(data))).first;
        }

        auto& imageData = *(imageDataIt->second);

        const auto& img = *debugMapEntry->image.get();

        if (img.size_.x() <= 0 || img.size_.y() <= 0)
        {
          continue;
        }

        jpegConv_.convert(img, imageData.data);

        imageData.width = img.size_.x();
        imageData.height = img.size_.y();
        imageData.length = imageData.key.length();

        imageData.hdr.msgType = DM_IMAGE;
        imageData.hdr.msgLength = sizeof(imageData.width) + sizeof(imageData.height) +
                                  sizeof(imageData.length) + imageData.length +
                                  imageData.data.size();

        sendBuffers_.push_back(boost::asio::buffer(&imageData.hdr, sizeof(DebugMessageHeader)));
        sendBuffers_.push_back(boost::asio::buffer(&imageData.width, sizeof(imageData.width)));
        sendBuffers_.push_back(boost::asio::buffer(&imageData.height, sizeof(imageData.height)));
        sendBuffers_.push_back(boost::asio::buffer(&imageData.length, sizeof(imageData.length)));
        sendBuffers_.push_back(boost::asio::buffer(imageData.key));
        sendBuffers_.push_back(boost::asio::buffer(imageData.data.data(), imageData.data.size()));
      }
    }
    lg.unlock();

    jsonData_ += "]";

    if (jsonData_.size() > 2)
    {
      // Create a Message Header containing the length of the json etc.
      headerToSend_.msgLength = jsonData_.length();
      headerToSend_.msgType = DM_UPDATE;

      // concatenate the buffers.
      sendBuffers_.push_back(boost::asio::buffer(&headerToSend_, sizeof(DebugMessageHeader)));
      sendBuffers_.push_back(boost::asio::buffer(jsonData_));
    }

    if (sendBuffers_.size() == 0)
    {
      canTransmit_.store(true);
      return;
    }

    try
    {
    // and send that stuff to the subscriber
#ifdef NAO
      auto self(shared_from_this());
#ifdef ITTNOTIFY_FOUND
      __itt_event_start(this->eventTransmission_);
#endif
      boost::asio::async_write(
          socket_, boost::make_iterator_range(sendBuffers_.begin(), sendBuffers_.end()),
          [this, self](boost::system::error_code error, std::size_t /*length*/) {
            canTransmit_.store(true);
            if (error)
            {
              Log(LogLevel::ERROR)
                  << "UnixSocketTransport: error while sending Updates, disconnecting. Error: "
                  << error.message();
              server_.disconnected(self);
              return;
            }
#ifdef ITTNOTIFY_FOUND
            __itt_event_end(this->eventTransmission_);
#endif

            Log(LogLevel::DEBUG) << "UnixSocketTransport: sent Updates.";
          });
#else
#ifdef ITTNOTIFY_FOUND
      __itt_event_start(this->eventTransmission_);
#endif
      boost::asio::write(socket_,
                         boost::make_iterator_range(sendBuffers_.begin(), sendBuffers_.end()));
      canTransmit_.store(true);
#ifdef ITTNOTIFY_FOUND
      __itt_event_end(this->eventTransmission_);
#endif
#endif
    }
    catch (boost::system::system_error&)
    {
      canTransmit_.store(true);
      Log(LogLevel::ERROR) << "Error transmitting debug updates!";
    }
  }
  catch (std::exception& e)
  {
    canTransmit_.store(true);
    Log(LogLevel::ERROR) << "Exception transmitting debug updates: " << e.what();
  }
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
{
  startAccept();
}

UnixSocketTransport::Impl::~Impl()
{
  sessions_.clear();
  ioService_.stop();
  backgroundThread_->join();
  socket_.close();
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

void UnixSocketTransport::Impl::transport()
{
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
  pimpl_ = std::make_unique<Impl>(file, debug);
  pimpl_->startBackgroundThread();
}

UnixSocketTransport::~UnixSocketTransport() {}

void UnixSocketTransport::transport()
{
  pimpl_->transport();
}

#endif
