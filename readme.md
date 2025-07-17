# jorge-a-day.fi
This is the backend that is used to run jorge-a-day.fi, a "feed" of images of my cat, Jorge!

The tech is mainly comprised of actix and a custom image cache.
To compile this project, you need to create a static directory and place favicon.ico inside of it. Otherwise actix will complain as the icon is baked in at compile time.

Images are compressed (webp) for the gallery view. Aside from that, no modifications are done to the input data. This is to say that exif data such as locations have to be cleared out of the images.
