#include <pthread.h>
#include <stdio.h>
#include <string.h>

#define NUM_THREADS 4
#define NUM_LOOP 100000

int entering[NUM_THREADS];
int tickets[NUM_THREADS];
int count = 0;

int read_volatile(volatile int *ptr) { return *ptr; }
void write_volatile(volatile int *ptr, int val) { *ptr = val; }

void lock_aqcuire(int idx) {
    __sync_synchronize();
    write_volatile(&entering[idx], 1);
    __sync_synchronize();

    int max = 0;
    for (int i = 0; i < NUM_THREADS; i++) {
        int t = read_volatile(&tickets[i]);
        if (t > max) {
            max = t;
        }
    }
    int ticket = max + 1;
    write_volatile(&tickets[idx], ticket);

    __sync_synchronize();
    write_volatile(&entering[idx], 0);
    __sync_synchronize();

    for (int i = 0; i < NUM_THREADS; i++) {
        if (i == idx) {
            continue;
        }

        __sync_synchronize();
        while (read_volatile(&entering[i])) {
        }
        __sync_synchronize();

        for (;;) {
            int t = read_volatile(&tickets[i]);
            if (t) {
                if (ticket < t || (ticket == t && idx < i))
                    break;
            } else {
                break;
            }
        }
    }

    __sync_synchronize();
}

void lock_release(int idx) {
    __sync_synchronize();
    write_volatile(&tickets[idx], 0);
    __sync_synchronize();
}

void *th(void *arg) {
    int idx = (int)arg;
    for (int i = 0; i < NUM_LOOP; i++) {
        lock_aqcuire(idx);
        int c = read_volatile(&count);
        write_volatile(&count, c + 1);
        lock_release(idx);
    }
    return NULL;
}

int main(int argc, char *argv[]) {
    pthread_t threads[NUM_THREADS];
    for (int i = 0; i < NUM_THREADS; i++) {
        if (pthread_create(&threads[i], NULL, th, (void *)i) != 0) {
            perror("pthread_create");
            return -1;
        }
    }

    for (int i = 0; i < NUM_THREADS; i++) {
        pthread_join(threads[i], NULL);
    }

    printf("count = %d (expected = %d)\n", count, NUM_LOOP * NUM_THREADS);

    return 0;
}