#include <stdio.h>
#include <unistd.h>

#include "wit.h"

void callback(char *result) {
    printf("Received result: %s\n", result);
    free(result);
    exit(0);
}

int main(int argc, char *argv[]) {
    struct wit_context *context = wit_init(NULL);
    wit_text_query_async(context, "hello", "ACCESS_TOKEN_HERE", callback);
    //wit_voice_query_auto_async(context, "ACCESS_TOKEN_HERE", callback);
    sleep(10);
    printf("Request timeout :(");
    return 0;
}
