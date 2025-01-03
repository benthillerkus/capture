#! /bin/python3

import os

os.system("""
    gst-launch-1.0 -v nvarguscamerasrc sensor_id=0 name=left nvarguscamerasrc sensor_id=1 name=right glstereomix name=mix
        left.  ! 'video/x-raw(memory:NVMM),width=(int)1920,height=(int)1080,format=(string)NV12,framerate=(fraction)30/1' ! nvvidconv ! glupload ! mix.
        right. ! 'video/x-raw(memory:NVMM),width=(int)1920,height=(int)1080,format=(string)NV12,framerate=(fraction)30/1' ! nvvidconv ! glupload ! mix. 
        mix.   ! 'video/x-raw(memory:GLMemory),multiview-mode=top-bottom' !
        glcolorconvert ! gldownload ! queue ! x264enc ! h264parse ! mp4mux ! progressreport ! filesink location=output.mp4 -e
        
""".replace("\n", ""))
