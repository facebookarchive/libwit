#include <stdio.h>
#include <unistd.h>

#include "wit.h"

struct wit_context *context;

void callback(char *result) {
    printf("Received result: %s\n", result);
    free(result);
    wit_close(context);
    exit(0);
}

int main(int argc, char *argv[]) {
    context = wit_init(NULL);
    wit_voice_query_auto_async(context, "ACCESS_TOKEN_HERE", callback);
    printf("Say something...\n");
    sleep(10);
    printf("Request timeout :(\n");
    return 1;
}
