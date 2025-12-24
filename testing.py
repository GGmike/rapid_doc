import rapid_pdf

items = rapid_pdf.extract_text_from_pdf("test.pdf")

# print(items)

origin_y = items[0].y

for item in items:
    if item.y != origin_y:
        print("\n")
        origin_y = item.y   
    print(item.text, end=' ')

change_result = rapid_pdf.replace_text_by_pos("test.pdf","testing_output.pdf", 1,
                                      "CORPORATE",
        "Testing",
        277.895, 451.84802,
        0.00, )
print("\nChange Result:", change_result)
    # print(item.text, item.x, item.y, item.font_size)