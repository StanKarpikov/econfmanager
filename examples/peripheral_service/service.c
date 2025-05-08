#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include "lib/econfmanager.h"

void update_callback(ParameterId id, void* arg)
{
    CInterfaceInstance* interface = (CInterfaceInstance*)arg;

    printf("Parameter updated: %u (arg: %p)\n", id, arg);
    switch(id)
    {
        case IMAGE_ACQUISITION_IMAGE_WIDTH:
            {
                image_acquisition_image_width_t image_acquisition_image_width;
                EconfStatus status = get_image_acquisition_image_width(interface, &image_acquisition_image_width);
                if (status == StatusOk) {
                    printf("Image width update: %d\n", image_acquisition_image_width);
                } else {
                    fprintf(stderr, "Failed to get image_acquisition_image_width\n");
                }
            }
            break;
        case IMAGE_ACQUISITION_EXPOSURE:
            {
                image_acquisition_exposure_t image_acquisition_exposure;
                EconfStatus status = get_image_acquisition_exposure(interface, &image_acquisition_exposure);
                if (status == StatusOk) {
                    printf("Exposure update: %0.2f\n", image_acquisition_exposure);
                } else {
                    fprintf(stderr, "Failed to get image_acquisition_exposure\n");
                }
            }
            break;
        default:
                break;
            }
}

int main(int argc, char *argv[]) {
    EconfStatus status;
    
    CInterfaceInstance* interface = {0};
    status = econf_init(
        "parameters.db",
        "saved_parameters.db",
        "default_data",
        &interface
    );
    
    if (status != StatusOk) {
        fprintf(stderr, "Failed to initialize configuration manager\n");
        return EXIT_FAILURE;
    }

    econf_set_up_timer_poll(interface, 5000);

    econf_add_callback(interface, IMAGE_ACQUISITION_IMAGE_WIDTH, update_callback, interface);
    if (status == StatusOk) {
        printf("Callback added for IMAGE_ACQUISITION_IMAGE_WIDTH\n");
    } else {
        fprintf(stderr, "Failed to add callback\n");
    }

    econf_add_callback(interface, IMAGE_ACQUISITION_EXPOSURE, update_callback, interface);
    if (status == StatusOk) {
        printf("Callback added for IMAGE_ACQUISITION_EXPOSURE\n");
    } else {
        fprintf(stderr, "Failed to add callback\n");
    }

    char serial_number[255] = {0};
    status = get_device_serial_number(interface, serial_number, sizeof(serial_number));
    if (status == StatusOk) {
        printf("Current serial number: %s\n", serial_number);
    } else {
        fprintf(stderr, "Failed to get serial number\n");
    }

    sprintf(serial_number, "new-serial-012345");
    status = set_device_serial_number(interface, serial_number);
    if (status != StatusOk) {
        fprintf(stderr, "Failed to set serial number\n");
    }
    else {
        fprintf(stderr, "Set okay\n");
    }

    printf("Enter 'q' to quit...\n");
    while(true)
    {
        char c = getchar();
        if (c == 'q') break;
        sleep(1);
    }

    printf("Exited.\n");
    return EXIT_SUCCESS;
}