// Bubble sort
let nums = [5, 1, 4, 2, 3, 6, 0]

function bubble_sort(arr) {
    for (let i = 0; i < arr.length; i++) {
        for (let j = i % 2; j < arr.length; j += 2) {
            if (arr[j] > arr[j + 1]) {
                let temp = arr[j]
                arr[j] = arr[j + 1]
                arr[j + 1] = temp
            }
        }
    }

    return arr
}


console.log(bubble_sort(nums))